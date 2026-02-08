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

        if (data.has_errors) {
            showErrorEditor(data);
        } else {
            showDashboard(data);
        }
    } catch (e) {
        uploadError.textContent = e.message;
        uploadError.hidden = false;
    }
}

function showDashboard(data) {
    // Populate summary bar
    document.getElementById('inv-name').textContent = data.name;
    document.getElementById('stat-plots').textContent = data.num_plots;
    document.getElementById('stat-trees').textContent = data.num_trees;
    document.getElementById('stat-species').textContent = data.species.length;

    // Show dashboard, hide others
    document.getElementById('upload-section').hidden = true;
    document.getElementById('error-editor').hidden = true;
    document.getElementById('dashboard').hidden = false;

    // Load data in parallel
    Promise.all([loadMetrics(), loadDistribution(), loadStatistics()])
        .then(() => runGrowth());
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

// ---------------------------------------------------------------------------
// Error Editor
// ---------------------------------------------------------------------------

function showErrorEditor(data) {
    document.getElementById('upload-section').hidden = true;
    document.getElementById('dashboard').hidden = true;
    document.getElementById('error-editor').hidden = false;

    renderErrorList(data.errors);
    renderEditTable(data.trees);
    highlightErrorCells(data.errors);
}

function renderErrorList(errors) {
    const el = document.getElementById('error-list');
    document.getElementById('error-count').textContent = errors.length + ' error' + (errors.length !== 1 ? 's' : '') + ' found';
    el.innerHTML = '';
    for (const e of errors) {
        const badge = document.createElement('span');
        badge.className = 'error-badge';
        badge.setAttribute('role', 'listitem');
        badge.title = 'Click to jump to this cell';
        const strong = document.createElement('strong');
        strong.textContent = 'Row ' + (e.row_index + 1);
        badge.appendChild(strong);
        badge.appendChild(document.createTextNode(
            ' Plot ' + e.plot_id + ', Tree ' + e.tree_id + ' \u2014 ' + e.field + ': ' + e.message
        ));
        // Click-to-jump: scroll to the error cell and highlight the row
        badge.addEventListener('click', () => jumpToErrorCell(e.row_index, e.field));
        el.appendChild(badge);
    }
}

function jumpToErrorCell(rowIndex, field) {
    const td = document.querySelector(
        `#edit-table-body tr[data-row="${rowIndex}"] td[data-field="${field}"]`
    );
    if (!td) return;
    const tr = td.closest('tr');
    // Scroll into view
    td.scrollIntoView({ behavior: 'smooth', block: 'center' });
    // Focus the input/select inside the cell
    const input = td.querySelector('input, select');
    if (input) setTimeout(() => input.focus(), 300);
    // Briefly highlight the row
    tr.classList.add('highlight-row');
    setTimeout(() => tr.classList.remove('highlight-row'), 1500);
}

const EDIT_FIELDS = [
    { key: 'plot_id', label: 'Plot ID', type: 'number' },
    { key: 'tree_id', label: 'Tree ID', type: 'number' },
    { key: 'species_code', label: 'Sp. Code', type: 'text' },
    { key: 'species_name', label: 'Species', type: 'text' },
    { key: 'dbh', label: 'DBH', type: 'number', step: '0.1' },
    { key: 'height', label: 'Height', type: 'number', step: '0.1', optional: true },
    { key: 'crown_ratio', label: 'Crown Ratio', type: 'number', step: '0.01', optional: true },
    { key: 'status', label: 'Status', type: 'select', options: ['Live', 'Dead', 'Cut', 'Missing'] },
    { key: 'expansion_factor', label: 'Exp. Factor', type: 'number', step: '0.1' },
    { key: 'age', label: 'Age', type: 'number', optional: true },
    { key: 'defect', label: 'Defect', type: 'number', step: '0.01', optional: true },
    { key: 'plot_size_acres', label: 'Plot Acres', type: 'number', step: '0.01', optional: true },
];

function renderEditTable(trees) {
    const thead = document.getElementById('edit-table-head');
    const tbody = document.getElementById('edit-table-body');

    thead.innerHTML = '<tr>' + EDIT_FIELDS.map(f => `<th>${f.label}</th>`).join('') + '</tr>';
    tbody.innerHTML = '';

    for (const tree of trees) {
        const tr = document.createElement('tr');
        tr.dataset.row = tree.row_index;
        // Preserve hidden fields that aren't shown in the table
        tr._hiddenFields = {
            slope_percent: tree.slope_percent,
            aspect_degrees: tree.aspect_degrees,
            elevation_ft: tree.elevation_ft,
        };

        for (const f of EDIT_FIELDS) {
            const td = document.createElement('td');
            td.dataset.field = f.key;
            td.dataset.label = f.label; // for mobile card layout

            const ariaLabel = f.label + ' for row ' + (tree.row_index + 1);

            if (f.type === 'select') {
                const sel = document.createElement('select');
                sel.dataset.row = tree.row_index;
                sel.dataset.field = f.key;
                sel.setAttribute('aria-label', ariaLabel);
                for (const opt of f.options) {
                    const o = document.createElement('option');
                    o.value = opt;
                    o.textContent = opt;
                    if (tree[f.key] === opt) o.selected = true;
                    sel.appendChild(o);
                }
                td.appendChild(sel);
            } else {
                const inp = document.createElement('input');
                inp.type = f.type;
                inp.dataset.row = tree.row_index;
                inp.dataset.field = f.key;
                inp.setAttribute('aria-label', ariaLabel);
                if (f.step) inp.step = f.step;
                const val = tree[f.key];
                inp.value = (val === null || val === undefined) ? '' : val;
                td.appendChild(inp);
            }

            tr.appendChild(td);
        }

        tbody.appendChild(tr);
    }
}

function highlightErrorCells(errors) {
    // Clear previous highlights
    document.querySelectorAll('.error-cell').forEach(el => el.classList.remove('error-cell'));

    for (const err of errors) {
        const td = document.querySelector(
            `#edit-table-body tr[data-row="${err.row_index}"] td[data-field="${err.field}"]`
        );
        if (td) td.classList.add('error-cell');
    }
}

function collectTableData() {
    const tbody = document.getElementById('edit-table-body');
    const rows = [];
    for (const tr of tbody.querySelectorAll('tr')) {
        const rowIdx = parseInt(tr.dataset.row);
        const row = { row_index: rowIdx };
        for (const f of EDIT_FIELDS) {
            const el = tr.querySelector(`[data-field="${f.key}"]`);
            if (!el) continue;
            const input = el.tagName === 'INPUT' || el.tagName === 'SELECT' ? el : el.querySelector('input, select');
            if (!input) continue;
            const val = input.value.trim();
            if (f.type === 'number') {
                if (val === '') {
                    row[f.key] = f.optional ? null : 0;
                } else {
                    const isInt = f.key === 'age' || f.key === 'plot_id' || f.key === 'tree_id';
                    const parsed = isInt ? parseInt(val, 10) : parseFloat(val);
                    // Preserve NaN as 0 only for required fields — server will catch
                    // the validation error for nonsensical values like 0 DBH
                    row[f.key] = Number.isNaN(parsed) ? 0 : parsed;
                }
            } else {
                row[f.key] = val;
            }
        }
        // Carry forward hidden fields from the original data
        row.slope_percent = tr._hiddenFields ? tr._hiddenFields.slope_percent : null;
        row.aspect_degrees = tr._hiddenFields ? tr._hiddenFields.aspect_degrees : null;
        row.elevation_ft = tr._hiddenFields ? tr._hiddenFields.elevation_ft : null;
        rows.push(row);
    }
    return rows;
}

async function revalidateData() {
    const btn = document.getElementById('validate-btn');
    btn.classList.add('loading');
    btn.disabled = true;

    const trees = collectTableData();
    try {
        const res = await fetch('/api/validate', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: currentId, trees })
        });
        if (!res.ok) {
            const err = await res.json();
            throw new Error(err.details || err.error);
        }
        const data = await res.json();
        currentId = data.id;

        if (data.has_errors) {
            renderErrorList(data.errors);
            highlightErrorCells(data.errors);
        } else {
            showDashboard(data);
        }
    } catch (e) {
        document.getElementById('error-count').textContent = 'Error: ' + e.message;
    } finally {
        btn.classList.remove('loading');
        btn.disabled = false;
    }
}

function startOver() {
    if (!confirm('Discard all edits and start over with a new file?')) return;
    document.getElementById('error-editor').hidden = true;
    document.getElementById('dashboard').hidden = true;
    document.getElementById('upload-section').hidden = false;
    fileInput.value = '';
    currentId = null;
}

// Expose runGrowth, exportData, and editor functions to HTML onclick handlers
window.runGrowth = runGrowth;
window.exportData = exportData;
window.revalidateData = revalidateData;
window.startOver = startOver;
