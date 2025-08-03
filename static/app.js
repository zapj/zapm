

async function toggleAddProcessForm() {
    document.getElementById('addProcess').classList.toggle('hidden');
}

// Fetch all processes
async function fetchProcesses() {
    try {
        const response = await fetch('/api/processes');
        const data = await response.json();
        
        const tableBody = document.querySelector('#processTable tbody');
        tableBody.innerHTML = '';
        
        Object.values(data).forEach(process => {
            const row = document.createElement('tr');
            
            const statusClass = 
                process.status === 'Running' ? 'status-running' : 
                process.status === 'Stopped' ? 'status-stopped' : 
                process.status === 'Failed' ? 'status-failed' : '';
            
            row.innerHTML = `
                <td>${process.name}</td>
                <td>${process.command}</td>
                <td class="${statusClass}">${process.status}</td>
                <td>${process.pid || '-'}</td>
                <td class="actions">
                    <button class="btn-start" data-name="${process.name}">Start</button>
                    <button class="btn-stop" data-name="${process.name}">Stop</button>
                    <button class="btn-restart" data-name="${process.name}">Restart</button>
                    <button class="btn-delete" data-name="${process.name}">Delete</button>
                </td>
            `;
            
            tableBody.appendChild(row);
        });
        
        // Add event listeners to buttons
        document.querySelectorAll('.btn-start').forEach(btn => {
            btn.addEventListener('click', () => startProcess(btn.dataset.name));
        });
        
        document.querySelectorAll('.btn-stop').forEach(btn => {
            btn.addEventListener('click', () => stopProcess(btn.dataset.name));
        });
        
        document.querySelectorAll('.btn-restart').forEach(btn => {
            btn.addEventListener('click', () => restartProcess(btn.dataset.name));
        });
        
        document.querySelectorAll('.btn-delete').forEach(btn => {
            btn.addEventListener('click', () => deleteProcess(btn.dataset.name));
        });
        
    } catch (error) {
        console.error('Error fetching processes:', error);
    }
}

// Start a process
async function startProcess(name) {
    try {
        await fetch(`/api/processes/${name}/start`, { method: 'POST', headers: {'Content-Type': 'application/json'} });
        fetchProcesses();
    } catch (error) {
        console.error(`Error starting process ${name}:`, error);
    }
}

// Stop a process
async function stopProcess(name) {
    try {
        await fetch(`/api/processes/${name}/stop`, { method: 'POST' });
        fetchProcesses();
    } catch (error) {
        console.error(`Error stopping process ${name}:`, error);
    }
}

// Restart a process
async function restartProcess(name) {
    try {
        await fetch(`/api/processes/${name}/restart`, { method: 'POST' });
        fetchProcesses();
    } catch (error) {
        console.error(`Error restarting process ${name}:`, error);
    }
}

// Delete a process
async function deleteProcess(name) {
    if (confirm(`Are you sure you want to delete process ${name}?`)) {
        try {
            await fetch(`/api/processes/${name}`, { method: 'DELETE' });
            fetchProcesses();
        } catch (error) {
            console.error(`Error deleting process ${name}:`, error);
        }
    }
}

// Add a new process
document.getElementById('addProcessForm').addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const name = document.getElementById('name').value;
    const command = document.getElementById('command').value;
    const workingDir = document.getElementById('workingDir').value;
    const envText = document.getElementById('env').value;
    const autoRestart = document.getElementById('autoRestart').checked;
    
    // Parse environment variables
    const env = {};
    if (envText) {
        envText.split('\n').forEach(line => {
            const [key, value] = line.split('=');
            if (key && value) {
                env[key.trim()] = value.trim();
            }
        });
    }
    
    const processData = {
        name,
        command,
        working_dir: workingDir || null,
        env: Object.keys(env).length > 0 ? env : null,
        auto_restart: autoRestart
    };
    
    try {
        await fetch(`/api/processes/${name}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(processData)
        });
        
        // Reset form
        document.getElementById('addProcessForm').reset();
        
        // Refresh process list
        fetchProcesses();
        
    } catch (error) {
        console.error('Error adding process:', error);
    }
});

// Initial fetch
fetchProcesses();

// Refresh every 5 seconds
setInterval(fetchProcesses, 5000);