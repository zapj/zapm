// 全局变量
let cpuChart;
let memoryChart;
let socket;
let logSocket;
let services = [];
let currentLogService = '';

// 初始化函数
function init() {
    setupWebSocket();
    loadServices();
    setupEventListeners();
}

// 设置WebSocket连接
function setupWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    socket = new WebSocket(`${protocol}//${host}/ws/stats`);

    socket.onmessage = function(event) {
        const stats = JSON.parse(event.data);
        updateServicesTable(stats);
    };

    socket.onclose = function() {
        setTimeout(setupWebSocket, 1000);
    };
}


// 加载服务列表
function loadServices() {
    fetch('/api/services')
        .then(response => response.json())
        .then(data => {
            services = data;
            updateServicesDropdown();
            updateServicesTable();
        });
}

// 更新服务下拉菜单
function updateServicesDropdown() {
    const select = document.getElementById('log-service-select');
    select.innerHTML = '<option value="">选择服务...</option>';
    
    services.forEach(service => {
        const option = document.createElement('option');
        option.value = service.name;
        option.textContent = service.name;
        select.appendChild(option);
    });
}

// 更新服务表格
function updateServicesTable(stats) {
    const tbody = document.getElementById('services-body');
    if (!tbody) {
        console.error('无法找到表格体元素');
        return;
    }

    // 清空现有内容
    while (tbody.firstChild) {
        tbody.removeChild(tbody.firstChild);
    }

    if (!services || services.length === 0) {
        const row = document.createElement('tr');
        const cell = document.createElement('td');
        cell.colSpan = 7;
        cell.textContent = '没有可用的服务数据';
        cell.className = 'text-center text-muted';
        row.appendChild(cell);
        tbody.appendChild(row);
        return;
    }

    services.forEach(service => {
        try {
            const stat = stats ? stats[service.name] : null;
            const row = document.createElement('tr');
            
            // 名称
            const nameCell = document.createElement('td');
            nameCell.textContent = service.name || '-';
            row.appendChild(nameCell);
            
            // 状态
            const statusCell = document.createElement('td');
            const status = stat ? stat.status : service.status;
            statusCell.textContent = status || 'unknown';
            statusCell.className = `status-${status}`;
            row.appendChild(statusCell);
            
            // PID
            const pidCell = document.createElement('td');
            pidCell.textContent = (stat && stat.pid) ? stat.pid : (service.pid || '-');
            row.appendChild(pidCell);
            
            // 运行时间
            const uptimeCell = document.createElement('td');
            uptimeCell.textContent = (stat && stat.uptime) ? formatUptime(stat.uptime) : '-';
            row.appendChild(uptimeCell);
            
            // CPU
            const cpuCell = document.createElement('td');
            cpuCell.textContent = (stat && stat.cpuUsage) ? `${stat.cpuUsage.toFixed(1)}%` : '-';
            row.appendChild(cpuCell);
            
            // 内存
            const memCell = document.createElement('td');
            memCell.textContent = (stat && stat.memoryUsage) ? formatMemory(stat.memoryUsage) : '-';
            row.appendChild(memCell);
            
            // 操作
            const actionCell = document.createElement('td');
            if (status === 'running') {
                actionCell.innerHTML = `
                    <button class="btn btn-danger btn-sm btn-action" onclick="stopService('${service.name}')">停止</button>
                    <button class="btn btn-warning btn-sm btn-action" onclick="restartService('${service.name}')">重启</button>
                `;
            } else {
                actionCell.innerHTML = `
                    <button class="btn btn-success btn-sm btn-action" onclick="startService('${service.name}')">启动</button>
                `;
            }
            
            row.appendChild(actionCell);
            tbody.appendChild(row);
        } catch (error) {
            console.error('渲染服务行时出错:', error);
        }
    });
}


// 设置事件监听器
function setupEventListeners() {
    document.getElementById('log-service-select').addEventListener('change', function() {
        const serviceName = this.value;
        if (serviceName) {
            fetchLogs(serviceName);
        } else {
            document.getElementById('log-content').textContent = '';
        }
    });
}

// 获取日志
function fetchLogs(serviceName) {
    const logContent = document.getElementById('log-content');
    logContent.textContent = '正在连接日志流...';
    
    // 关闭之前的日志WebSocket连接
    if (logSocket && logSocket.readyState === WebSocket.OPEN) {
        logSocket.close();
    }
    
    // 创建新的WebSocket连接
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/api/stream-logs?service=${serviceName}`;
    logSocket = new WebSocket(wsUrl);
    
    // 自动滚动控制
    let autoScroll = true;
    
    // 添加自动滚动控制按钮
    const logContainer = document.getElementById('log-container');
    let scrollControl = document.getElementById('scroll-control');
    if (!scrollControl) {
        scrollControl = document.createElement('div');
        scrollControl.id = 'scroll-control';
        scrollControl.className = 'scroll-control';
        scrollControl.innerHTML = `
            <label>
                <input type="checkbox" id="auto-scroll" checked> 自动滚动
            </label>
        `;
        logContainer.insertBefore(scrollControl, logContent);
        
        // 监听自动滚动复选框
        document.getElementById('auto-scroll').addEventListener('change', function() {
            autoScroll = this.checked;
        });
    }
    
    // WebSocket事件处理
    logSocket.onopen = function() {
        console.log(`已连接到 ${serviceName} 的日志流`);
        logContent.textContent = '';
    };
    
    // 限制最大显示行数
    const maxLines = 1000;
    let logLines = [];

    logSocket.onmessage = function(event) {
        // 处理接收到的日志数据
        const newLines = event.data.split('\n');
        
        // 添加新的日志行
        logLines.push(...newLines.filter(line => line.trim() !== ''));
        
        // 如果超过最大行数，删除旧的行
        if (logLines.length > maxLines) {
            logLines = logLines.slice(-maxLines);
        }
        
        // 更新显示
        logContent.textContent = logLines.join('\n');
        
        // 如果启用了自动滚动，则滚动到底部
        if (autoScroll) {
            logContent.scrollTop = logContent.scrollHeight;
        }
    };
    
    logSocket.onerror = function(error) {
        console.error('WebSocket错误:', error);
        logContent.textContent += '\n[错误] WebSocket连接出错\n';
    };
    
    logSocket.onclose = function() {
        console.log('日志WebSocket连接已关闭');
        if (logContent.textContent === '' || logContent.textContent === '正在连接日志流...') {
            logContent.textContent = '无日志数据或连接已关闭';
        }
    };
}

// 格式化运行时间
function formatUptime(seconds) {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    
    let result = '';
    if (days > 0) result += `${days}d `;
    if (hours > 0 || days > 0) result += `${hours}h `;
    if (mins > 0 || hours > 0 || days > 0) result += `${mins}m `;
    result += `${secs}s`;
    
    return result;
}

// 格式化内存
function formatMemory(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

// 服务操作函数
function startService(name) {
    fetch(`/api/service/${name}/start`, { method: 'POST' })
        .then(response => {
            if (response.ok) {
                loadServices();
            }
        });
}

function stopService(name) {
    fetch(`/api/service/${name}/stop`, { method: 'POST' })
        .then(response => {
            if (response.ok) {
                loadServices();
            }
        });
}

function restartService(name) {
    fetch(`/api/service/${name}/restart`, { method: 'POST' })
        .then(response => {
            if (response.ok) {
                loadServices();
            }
        });
}

// 页面加载完成后初始化
document.addEventListener('DOMContentLoaded', init);