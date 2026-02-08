/* Forest Inventory Analyzer - Web UI */

let currentId = null;
let speciesChart = null;
let diameterChart = null;
let growthChart = null;

// ---------------------------------------------------------------------------
// Upload handling
// ---------------------------------------------------------------------------

const dropZone = document.getElementById('drop-zone');
const fileInput = document.getElementById('file-input');
const uploadError = document.getElementById('upload-error');

dropZone.addEventListener('click', () => fileInput.click());

dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('drag-over');
});

dropZone.addEventListener('dragleave', () => {
    dropZone.classList.remove('drag-over');
});

dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
    if (e.dataTransfer.files.length) {
        uploadFile(e.dataTransfer.files[0]);
    }
});

fileInput.addEventListener('change', () => {
    if (fileInput.files.length) {
        uploadFile(fileInput.files[0]);
    }
});

async function uploadFile(file) {
    uploadError.hidden = true;
    const formData = new FormData();
    formData.append('file', file);

    try {
        const res = await fetch('/api/upload', { method: 'POST', body: formData });
        if (!res.ok) {
            const err = await res.json();
            throw new Error(err.details || err.error);
        }
        const data = await res.json();
        currentId = data.id;

        // Populate summary bar
        document.getElementById('inv-name').textContent = data.name;
        document.getElementById('stat-plots').textContent = data.num_plots;
        document.getElementById('stat-trees').textContent = data.num_trees;
        document.getElementById('stat-species').textContent = data.species.length;

        // Show dashboard, hide upload
        document.getElementById('upload-section').hidden = true;
        document.getElementById('dashboard').hidden = false;

        // Load data in parallel
        await Promise.all([loadMetrics(), loadDistribution(), loadStatistics()]);
        // Auto-run growth with defaults
        await runGrowth();
    } catch (e) {
        uploadError.textContent = e.message;
        uploadError.hidden = false;
    }
}

// ---------------------------------------------------------------------------
// API helpers
// ---------------------------------------------------------------------------

async function apiFetch(path) {
    const res = await fetch(path);
    if (!res.ok) {
        const err = await res.json();
        throw new Error(err.details || err.error);
    }
    return res.json();
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

async function loadMetrics() {
    try {
        const m = await apiFetch(`/api/${currentId}/metrics`);
        document.getElementById('metric-tpa').textContent = m.total_tpa.toFixed(1);
        document.getElementById('metric-ba').textContent = m.total_basal_area.toFixed(1);
        document.getElementById('metric-vol-cuft').textContent = m.total_volume_cuft.toFixed(1);
        document.getElementById('metric-vol-bdft').textContent = m.total_volume_bdft.toFixed(0);
        document.getElementById('metric-qmd').textContent = m.quadratic_mean_diameter.toFixed(1);

        // Species composition chart
        renderSpeciesChart(m.species_composition);
    } catch (e) {
        console.error('Metrics error:', e);
    }
}

// ---------------------------------------------------------------------------
// Species Chart
// ---------------------------------------------------------------------------

function renderSpeciesChart(composition) {
    const ctx = document.getElementById('species-chart').getContext('2d');
    if (speciesChart) speciesChart.destroy();

    const labels = composition.map(s => s.species.common_name);
    const baData = composition.map(s => s.basal_area);
    const tpaData = composition.map(s => s.tpa);

    const greens = [
        '#2d5016', '#3d6b20', '#4d8030', '#5e9540', '#6faa50',
        '#8fbc5a', '#a0cc6a', '#b5d98a', '#c8e4a0', '#ddf0c0'
    ];

    speciesChart = new Chart(ctx, {
        type: 'bar',
        data: {
            labels,
            datasets: [
                {
                    label: 'Basal Area (ft\u00b2/ac)',
                    data: baData,
                    backgroundColor: greens.slice(0, labels.length),
                },
                {
                    label: 'TPA',
                    data: tpaData,
                    backgroundColor: greens.slice(0, labels.length).map(c => c + '88'),
                }
            ]
        },
        options: {
            indexAxis: 'y',
            responsive: true,
            plugins: {
                legend: { position: 'bottom' }
            },
            scales: {
                x: { beginAtZero: true }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Diameter Distribution
// ---------------------------------------------------------------------------

async function loadDistribution() {
    try {
        const dist = await apiFetch(`/api/${currentId}/distribution?class_width=2.0`);
        renderDiameterChart(dist);
    } catch (e) {
        console.error('Distribution error:', e);
    }
}

function renderDiameterChart(dist) {
    const ctx = document.getElementById('diameter-chart').getContext('2d');
    if (diameterChart) diameterChart.destroy();

    const labels = dist.classes.map(c => c.midpoint.toFixed(1) + '"');
    const tpaData = dist.classes.map(c => c.tpa);
    const baData = dist.classes.map(c => c.basal_area);

    diameterChart = new Chart(ctx, {
        type: 'bar',
        data: {
            labels,
            datasets: [
                {
                    label: 'TPA',
                    data: tpaData,
                    backgroundColor: '#8fbc5a',
                    yAxisID: 'y',
                },
                {
                    label: 'Basal Area (ft\u00b2/ac)',
                    data: baData,
                    backgroundColor: '#2d5016',
                    yAxisID: 'y1',
                }
            ]
        },
        options: {
            responsive: true,
            plugins: {
                legend: { position: 'bottom' }
            },
            scales: {
                y: {
                    beginAtZero: true,
                    position: 'left',
                    title: { display: true, text: 'TPA' }
                },
                y1: {
                    beginAtZero: true,
                    position: 'right',
                    grid: { drawOnChartArea: false },
                    title: { display: true, text: 'BA (ft\u00b2/ac)' }
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

async function loadStatistics() {
    const statsError = document.getElementById('stats-error');
    const statsTable = document.getElementById('stats-table');
    statsError.hidden = true;
    statsTable.hidden = false;

    try {
        const stats = await apiFetch(`/api/${currentId}/statistics?confidence=0.95`);
        const tbody = document.getElementById('stats-body');
        tbody.innerHTML = '';

        const rows = [
            ['Trees per Acre', stats.tpa],
            ['Basal Area (ft\u00b2/ac)', stats.basal_area],
            ['Volume (ft\u00b3/ac)', stats.volume_cuft],
            ['Volume (bd ft/ac)', stats.volume_bdft],
        ];

        for (const [name, ci] of rows) {
            const tr = document.createElement('tr');
            tr.innerHTML = `
                <td>${name}</td>
                <td>${ci.mean.toFixed(2)}</td>
                <td>${ci.std_error.toFixed(2)}</td>
                <td>${ci.lower.toFixed(2)}</td>
                <td>${ci.upper.toFixed(2)}</td>
                <td>${ci.sampling_error_percent.toFixed(1)}%</td>
            `;
            tbody.appendChild(tr);
        }
    } catch (e) {
        statsError.textContent = e.message;
        statsError.hidden = false;
        statsTable.hidden = true;
    }
}

// ---------------------------------------------------------------------------
// Growth Projection
// ---------------------------------------------------------------------------

async function runGrowth() {
    if (!currentId) return;

    const modelName = document.getElementById('growth-model').value;
    const years = parseInt(document.getElementById('growth-years').value) || 20;
    const rate = parseFloat(document.getElementById('growth-rate').value) || 0.03;
    const capacity = parseFloat(document.getElementById('growth-capacity').value) || 300;
    const mortality = parseFloat(document.getElementById('growth-mortality').value) || 0.005;

    let model;
    switch (modelName) {
        case 'Exponential':
            model = { Exponential: { annual_rate: rate, mortality_rate: mortality } };
            break;
        case 'Logistic':
            model = { Logistic: { annual_rate: rate, carrying_capacity: capacity, mortality_rate: mortality } };
            break;
        case 'Linear':
            model = { Linear: { annual_increment: rate, mortality_rate: mortality } };
            break;
    }

    try {
        const res = await fetch(`/api/${currentId}/growth`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ model, years })
        });
        if (!res.ok) {
            const err = await res.json();
            throw new Error(err.details || err.error);
        }
        const projections = await res.json();
        renderGrowthChart(projections);
    } catch (e) {
        console.error('Growth error:', e);
    }
}

function renderGrowthChart(projections) {
    const ctx = document.getElementById('growth-chart').getContext('2d');
    if (growthChart) growthChart.destroy();

    const labels = projections.map(p => 'Year ' + p.year);

    growthChart = new Chart(ctx, {
        type: 'line',
        data: {
            labels,
            datasets: [
                {
                    label: 'TPA',
                    data: projections.map(p => p.tpa),
                    borderColor: '#8fbc5a',
                    backgroundColor: '#8fbc5a22',
                    yAxisID: 'y',
                    tension: 0.3,
                },
                {
                    label: 'Basal Area (ft\u00b2/ac)',
                    data: projections.map(p => p.basal_area),
                    borderColor: '#2d5016',
                    backgroundColor: '#2d501622',
                    yAxisID: 'y1',
                    tension: 0.3,
                },
                {
                    label: 'Volume (ft\u00b3/ac)',
                    data: projections.map(p => p.volume_cuft),
                    borderColor: '#6faa50',
                    backgroundColor: '#6faa5022',
                    yAxisID: 'y1',
                    borderDash: [5, 5],
                    tension: 0.3,
                }
            ]
        },
        options: {
            responsive: true,
            interaction: {
                mode: 'index',
                intersect: false,
            },
            plugins: {
                legend: { position: 'bottom' }
            },
            scales: {
                y: {
                    beginAtZero: true,
                    position: 'left',
                    title: { display: true, text: 'TPA' }
                },
                y1: {
                    beginAtZero: true,
                    position: 'right',
                    grid: { drawOnChartArea: false },
                    title: { display: true, text: 'BA / Volume' }
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

function exportData(format) {
    if (!currentId) return;
    window.location.href = `/api/${currentId}/export?format=${format}`;
}

// Expose runGrowth and exportData to HTML onclick handlers
window.runGrowth = runGrowth;
window.exportData = exportData;
