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
const uploadProgress = document.getElementById('upload-progress');

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
    document.getElementById('upload-filename').textContent = file.name;
    uploadProgress.classList.add('active');
    dropZone.style.display = 'none';

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
        dropZone.style.display = '';
    } finally {
        uploadProgress.classList.remove('active');
    }
}

function showDashboard(data) {
    document.getElementById('inv-name').textContent = data.name;
    document.getElementById('stat-plots').textContent = data.num_plots;
    document.getElementById('stat-trees').textContent = data.num_trees;
    document.getElementById('stat-species').textContent = data.species.length;

    document.getElementById('upload-section').hidden = true;
    document.getElementById('error-editor').hidden = true;
    document.getElementById('dashboard').hidden = false;
    document.getElementById('btn-new-analysis').classList.add('visible');

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
// Number formatting
// ---------------------------------------------------------------------------

function fmtNum(n, decimals) {
    if (n === null || n === undefined) return '--';
    if (Math.abs(n) >= 1000) {
        return n.toLocaleString('en-US', {
            minimumFractionDigits: decimals,
            maximumFractionDigits: decimals,
        });
    }
    return n.toFixed(decimals);
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

async function loadMetrics() {
    try {
        const m = await apiFetch(`/api/${currentId}/metrics`);
        document.getElementById('metric-tpa').textContent = fmtNum(m.total_tpa, 1);
        document.getElementById('metric-ba').textContent = fmtNum(m.total_basal_area, 1);
        document.getElementById('metric-vol-cuft').textContent = fmtNum(m.total_volume_cuft, 0);
        document.getElementById('metric-vol-bdft').textContent = fmtNum(m.total_volume_bdft, 0);
        document.getElementById('metric-qmd').textContent = fmtNum(m.quadratic_mean_diameter, 1);

        renderSpeciesChart(m.species_composition);

        // Show per-stand summary if cruise data has multiple stands
        const warning = document.getElementById('stand-summary-banner');
        if (m.stands && m.stands.length > 1) {
            const lines = m.stands.map(s =>
                `<strong>Stand ${s.stand_id}</strong> (${s.num_plots} plots, ${s.num_trees} trees): ` +
                `${fmtNum(s.tpa, 1)} TPA, ${fmtNum(s.basal_area, 1)} BA, ` +
                `${fmtNum(s.qmd, 1)}" QMD, ${fmtNum(s.volume_bdft, 0)} bd ft/ac`
            );
            warning.innerHTML = '<strong>Per-Stand Summary</strong><br>' + lines.join('<br>');
            warning.classList.add('visible');
        } else {
            warning.classList.remove('visible');
        }
    } catch (e) {
        console.error('Metrics error:', e);
    }
}

// ---------------------------------------------------------------------------
// Species Chart
// ---------------------------------------------------------------------------

const CHART_COLORS = [
    '#1b4332', '#2d6a4f', '#40916c', '#52b788', '#74c69d',
    '#95d5b2', '#b7e4c7', '#d8f3dc', '#a7c957', '#6a994e',
];

function renderSpeciesChart(composition) {
    const ctx = document.getElementById('species-chart').getContext('2d');
    if (speciesChart) speciesChart.destroy();

    const labels = composition.map(s => s.species.common_name);
    const baData = composition.map(s => s.basal_area);
    const tpaData = composition.map(s => s.tpa);

    speciesChart = new Chart(ctx, {
        type: 'bar',
        data: {
            labels,
            datasets: [
                {
                    label: 'Basal Area (ft\u00b2/ac)',
                    data: baData,
                    backgroundColor: CHART_COLORS.slice(0, labels.length),
                    borderRadius: 4,
                },
                {
                    label: 'TPA',
                    data: tpaData,
                    backgroundColor: CHART_COLORS.slice(0, labels.length).map(c => c + '66'),
                    borderRadius: 4,
                }
            ]
        },
        options: {
            indexAxis: 'y',
            responsive: true,
            plugins: {
                legend: {
                    position: 'bottom',
                    labels: { font: { family: 'Inter', size: 11 }, padding: 16 }
                }
            },
            scales: {
                x: {
                    beginAtZero: true,
                    grid: { color: '#f0f0f0' },
                    ticks: { font: { family: 'Inter', size: 11 } }
                },
                y: {
                    ticks: { font: { family: 'Inter', size: 11 } }
                }
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

    const labels = dist.classes.map(c => c.midpoint.toFixed(0) + '"');
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
                    backgroundColor: '#74c69d',
                    borderRadius: 3,
                    yAxisID: 'y',
                },
                {
                    label: 'BA (ft\u00b2/ac)',
                    data: baData,
                    backgroundColor: '#1b4332',
                    borderRadius: 3,
                    yAxisID: 'y1',
                }
            ]
        },
        options: {
            responsive: true,
            plugins: {
                legend: {
                    position: 'bottom',
                    labels: { font: { family: 'Inter', size: 11 }, padding: 16 }
                }
            },
            scales: {
                x: {
                    grid: { display: false },
                    ticks: { font: { family: 'Inter', size: 10 }, maxRotation: 45 }
                },
                y: {
                    beginAtZero: true,
                    position: 'left',
                    title: { display: true, text: 'TPA', font: { family: 'Inter', size: 11 } },
                    grid: { color: '#f0f0f0' },
                },
                y1: {
                    beginAtZero: true,
                    position: 'right',
                    grid: { drawOnChartArea: false },
                    title: { display: true, text: 'BA (ft\u00b2/ac)', font: { family: 'Inter', size: 11 } },
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

    const confidence = document.getElementById('confidence-select').value;

    try {
        const stats = await apiFetch(`/api/${currentId}/statistics?confidence=${confidence}`);
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
            const pct = ci.sampling_error_percent;
            const pctClass = pct > 20 ? ' style="color:var(--error);font-weight:600"' : '';
            tr.innerHTML =
                `<td>${name}</td>` +
                `<td>${fmtNum(ci.mean, 1)}</td>` +
                `<td>${fmtNum(ci.std_error, 2)}</td>` +
                `<td>${fmtNum(ci.lower, 1)}</td>` +
                `<td>${fmtNum(ci.upper, 1)}</td>` +
                `<td${pctClass}>${pct.toFixed(1)}%</td>`;
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
                    borderColor: '#74c69d',
                    backgroundColor: '#74c69d18',
                    fill: true,
                    yAxisID: 'y',
                    tension: 0.3,
                    pointRadius: 2,
                    borderWidth: 2,
                },
                {
                    label: 'Basal Area (ft\u00b2/ac)',
                    data: projections.map(p => p.basal_area),
                    borderColor: '#1b4332',
                    backgroundColor: '#1b433218',
                    fill: true,
                    yAxisID: 'y1',
                    tension: 0.3,
                    pointRadius: 2,
                    borderWidth: 2,
                },
                {
                    label: 'Volume (ft\u00b3/ac)',
                    data: projections.map(p => p.volume_cuft),
                    borderColor: '#52b788',
                    backgroundColor: '#52b78818',
                    fill: true,
                    yAxisID: 'y1',
                    borderDash: [5, 5],
                    tension: 0.3,
                    pointRadius: 2,
                    borderWidth: 2,
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
                legend: {
                    position: 'bottom',
                    labels: { font: { family: 'Inter', size: 11 }, padding: 16 }
                },
                tooltip: {
                    backgroundColor: '#1a1a2e',
                    titleFont: { family: 'Inter' },
                    bodyFont: { family: 'Inter' },
                    cornerRadius: 8,
                    padding: 10,
                }
            },
            scales: {
                x: {
                    grid: { display: false },
                    ticks: { font: { family: 'Inter', size: 10 } }
                },
                y: {
                    beginAtZero: true,
                    position: 'left',
                    title: { display: true, text: 'TPA', font: { family: 'Inter', size: 11 } },
                    grid: { color: '#f0f0f0' },
                },
                y1: {
                    beginAtZero: true,
                    position: 'right',
                    grid: { drawOnChartArea: false },
                    title: { display: true, text: 'BA / Volume', font: { family: 'Inter', size: 11 } },
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
    document.getElementById('btn-new-analysis').classList.add('visible');
    document.getElementById('autofix-banner').hidden = true;

    renderErrorList(data.errors);
    renderEditTable(data.trees);
    highlightErrorCells(data.errors);

    // Hint to the user that auto-fix may help
    const fixablePatterns = [
        /negative/i, /must be positive/i, /must be in 0\.0/i,
        /Unknown tree status/i,
    ];
    const fixable = data.errors.filter(e =>
        fixablePatterns.some(p => p.test(e.message))
    ).length;
    const hint = document.getElementById('autofix-hint');
    if (fixable > 0) {
        hint.textContent = fixable + ' issue' + (fixable !== 1 ? 's' : '') +
            ' may be auto-fixable \u2014 try "Auto-Fix Issues" above';
        hint.hidden = false;
    } else {
        hint.hidden = true;
    }
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
    td.scrollIntoView({ behavior: 'smooth', block: 'center' });
    const input = td.querySelector('input, select');
    if (input) setTimeout(() => input.focus(), 300);
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
        tr._hiddenFields = {
            slope_percent: tree.slope_percent,
            aspect_degrees: tree.aspect_degrees,
            elevation_ft: tree.elevation_ft,
        };

        for (const f of EDIT_FIELDS) {
            const td = document.createElement('td');
            td.dataset.field = f.key;
            td.dataset.label = f.label;

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
                    row[f.key] = Number.isNaN(parsed) ? 0 : parsed;
                }
            } else {
                row[f.key] = val;
            }
        }
        row.slope_percent = tr._hiddenFields ? tr._hiddenFields.slope_percent : null;
        row.aspect_degrees = tr._hiddenFields ? tr._hiddenFields.aspect_degrees : null;
        row.elevation_ft = tr._hiddenFields ? tr._hiddenFields.elevation_ft : null;
        rows.push(row);
    }
    return rows;
}

// ---------------------------------------------------------------------------
// Auto-fix: preview-and-select workflow
// ---------------------------------------------------------------------------

// Holds the last autofix response so we can apply selected fixes
let pendingAutofixData = null;
// Snapshot of table data before fixes were applied (for undo)
let preFixSnapshot = null;

// Categorize a fix by its field/reason into a human-readable group
function fixCategory(fix) {
    if (fix.field === 'species_code' || fix.field === 'species_name') return 'whitespace';
    if (fix.field === 'status') return 'status';
    if (fix.reason.toLowerCase().includes('negative') || fix.reason.toLowerCase().includes('absolute')) return 'signs';
    if (fix.reason.toLowerCase().includes('percentage') || fix.reason.toLowerCase().includes('proportion')) return 'units';
    if (fix.field === 'height') return 'heights';
    if (fix.field === 'dbh' && fix.reason.toLowerCase().includes('centimeter')) return 'dbh_units';
    return 'other';
}

const CATEGORY_LABELS = {
    whitespace: 'Whitespace & Case',
    status: 'Status Normalization',
    signs: 'Sign Corrections',
    units: 'Unit Conversions (% \u2192 proportion)',
    heights: 'Height Corrections',
    dbh_units: 'DBH Unit Conversions (cm \u2192 in)',
    other: 'Other Fixes',
};

const CONFIDENCE_COLORS = {
    high: '#2b8a3e',
    medium: '#e67700',
    low: '#c92a2a',
};

async function autofixData() {
    const btn = document.getElementById('autofix-btn');
    btn.classList.add('loading');
    btn.disabled = true;

    const trees = collectTableData();
    try {
        const res = await fetch('/api/autofix', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: currentId, trees })
        });
        if (!res.ok) {
            const err = await res.json();
            throw new Error(err.details || err.error);
        }
        const data = await res.json();
        pendingAutofixData = data;

        if (data.fixes.length === 0 && data.warnings.length === 0) {
            showAutofixBanner('No auto-fixable issues found. Please correct the remaining errors manually.', 'info');
            return;
        }

        // Show the preview panel
        renderAutofixPreview(data);
    } catch (e) {
        showAutofixBanner('Auto-fix error: ' + e.message, 'error');
    } finally {
        btn.classList.remove('loading');
        btn.disabled = false;
    }
}

function renderAutofixPreview(data) {
    const preview = document.getElementById('autofix-preview');
    const catContainer = document.getElementById('autofix-categories');
    const warningsPanel = document.getElementById('autofix-warnings-panel');
    const banner = document.getElementById('autofix-banner');
    banner.hidden = true;

    // Group fixes by category
    const groups = {};
    data.fixes.forEach((fix, idx) => {
        fix._idx = idx; // track original index
        const cat = fixCategory(fix);
        if (!groups[cat]) groups[cat] = [];
        groups[cat].push(fix);
    });

    catContainer.innerHTML = '';

    for (const [cat, fixes] of Object.entries(groups)) {
        const section = document.createElement('div');
        section.className = 'autofix-cat';

        // Category header with toggle
        const header = document.createElement('div');
        header.className = 'autofix-cat-header';
        const toggle = document.createElement('input');
        toggle.type = 'checkbox';
        toggle.checked = true;
        toggle.className = 'autofix-cat-toggle';
        toggle.dataset.category = cat;
        toggle.addEventListener('change', () => {
            section.querySelectorAll('.autofix-fix-cb').forEach(cb => {
                cb.checked = toggle.checked;
            });
            updateSelectionCount();
        });
        const label = document.createElement('label');
        label.className = 'autofix-cat-label';
        label.textContent = (CATEGORY_LABELS[cat] || cat) + ' (' + fixes.length + ')';
        label.prepend(toggle);
        header.appendChild(label);
        section.appendChild(header);

        // Individual fix rows
        const list = document.createElement('div');
        list.className = 'autofix-fix-list';
        for (const fix of fixes) {
            const row = document.createElement('label');
            row.className = 'autofix-fix-row';

            const cb = document.createElement('input');
            cb.type = 'checkbox';
            cb.className = 'autofix-fix-cb';
            cb.dataset.fixIdx = fix._idx;
            // Default: high & medium checked, low unchecked
            cb.checked = fix.confidence !== 'low';
            cb.addEventListener('change', () => {
                updateCategoryToggle(section, cat);
                updateSelectionCount();
            });

            const conf = document.createElement('span');
            conf.className = 'autofix-confidence autofix-conf-' + fix.confidence;
            conf.textContent = fix.confidence;
            conf.title = fix.confidence + ' confidence';

            const desc = document.createElement('span');
            desc.className = 'autofix-fix-desc';
            desc.innerHTML =
                '<strong>Row ' + (fix.row_index + 1) + '</strong> ' +
                '<code>' + fix.field + '</code>: ' +
                '<span class="autofix-old">' + escHtml(fix.original) + '</span>' +
                ' \u2192 ' +
                '<span class="autofix-new">' + escHtml(fix.fixed) + '</span>';

            const reason = document.createElement('span');
            reason.className = 'autofix-fix-reason';
            reason.textContent = fix.reason;

            const jumpBtn = document.createElement('button');
            jumpBtn.className = 'btn-sm btn-link autofix-jump';
            jumpBtn.textContent = 'Go to cell';
            jumpBtn.title = 'Navigate to this cell';
            jumpBtn.addEventListener('click', (e) => {
                e.preventDefault();
                jumpToErrorCell(fix.row_index, fix.field);
            });

            row.appendChild(cb);
            row.appendChild(conf);
            row.appendChild(desc);
            row.appendChild(reason);
            row.appendChild(jumpBtn);
            list.appendChild(row);
        }
        section.appendChild(list);
        catContainer.appendChild(section);
    }

    // Warnings panel
    if (data.warnings.length > 0) {
        warningsPanel.hidden = false;
        warningsPanel.innerHTML = '<h4>Warnings (cannot be auto-fixed)</h4>';
        const ul = document.createElement('ul');
        for (const w of data.warnings) {
            const li = document.createElement('li');
            li.className = 'autofix-warning-item';
            li.innerHTML =
                '<strong>Row ' + (w.row_index + 1) + '</strong> ' +
                '<code>' + w.field + '</code> = ' +
                escHtml(w.value) + ': ' + escHtml(w.message);
            li.style.cursor = 'pointer';
            li.addEventListener('click', () => jumpToErrorCell(w.row_index, w.field));
            ul.appendChild(li);
        }
        warningsPanel.appendChild(ul);
    } else {
        warningsPanel.hidden = true;
    }

    updateSelectionCount();
    preview.hidden = false;
    preview.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

function updateCategoryToggle(section, cat) {
    const cbs = section.querySelectorAll('.autofix-fix-cb');
    const allChecked = [...cbs].every(cb => cb.checked);
    const noneChecked = [...cbs].every(cb => !cb.checked);
    const toggle = section.querySelector('.autofix-cat-toggle');
    toggle.checked = allChecked;
    toggle.indeterminate = !allChecked && !noneChecked;
}

function updateSelectionCount() {
    const all = document.querySelectorAll('.autofix-fix-cb');
    const checked = document.querySelectorAll('.autofix-fix-cb:checked');
    const el = document.getElementById('autofix-selection-count');
    el.textContent = checked.length + ' of ' + all.length + ' selected';

    const applyBtn = document.getElementById('autofix-apply-btn');
    applyBtn.disabled = checked.length === 0;
    applyBtn.textContent = checked.length === 0
        ? 'Apply Selected Fixes'
        : 'Apply ' + checked.length + ' Fix' + (checked.length !== 1 ? 'es' : '');
}

function autofixSelectByConfidence(level) {
    const cbs = document.querySelectorAll('.autofix-fix-cb');
    cbs.forEach(cb => {
        const idx = parseInt(cb.dataset.fixIdx);
        const fix = pendingAutofixData.fixes[idx];
        if (level === 'all') {
            cb.checked = true;
        } else if (level === 'none') {
            cb.checked = false;
        } else {
            const allowed = level.split(',');
            cb.checked = allowed.includes(fix.confidence);
        }
    });
    // Update all category toggles
    document.querySelectorAll('.autofix-cat').forEach(section => {
        const cat = section.querySelector('.autofix-cat-toggle').dataset.category;
        updateCategoryToggle(section, cat);
    });
    updateSelectionCount();
}

function applySelectedFixes() {
    if (!pendingAutofixData) return;

    // Snapshot for undo
    preFixSnapshot = collectTableData();

    // Gather selected fix indices
    const selected = new Set();
    document.querySelectorAll('.autofix-fix-cb:checked').forEach(cb => {
        selected.add(parseInt(cb.dataset.fixIdx));
    });

    if (selected.size === 0) return;

    // Apply selected fixes to the table cells directly
    const applied = [];
    for (const idx of selected) {
        const fix = pendingAutofixData.fixes[idx];
        const td = document.querySelector(
            `#edit-table-body tr[data-row="${fix.row_index}"] td[data-field="${fix.field}"]`
        );
        if (!td) continue;
        const input = td.querySelector('input, select');
        if (!input) continue;

        input.value = fix.fixed;
        // Trigger change for any listeners
        input.dispatchEvent(new Event('change', { bubbles: true }));

        // Highlight the cell by confidence
        td.classList.remove('error-cell', 'fixed-cell', 'fixed-cell-review');
        td.classList.add(fix.confidence === 'high' ? 'fixed-cell' : 'fixed-cell-review');
        applied.push(fix);
    }

    // Close preview, show summary banner
    closeAutofixPreview();

    const skipped = pendingAutofixData.fixes.length - applied.length;
    let msg = applied.length + ' fix' + (applied.length !== 1 ? 'es' : '') + ' applied';
    if (skipped > 0) msg += ' (' + skipped + ' skipped)';
    if (pendingAutofixData.warnings.length > 0) {
        msg += '. ' + pendingAutofixData.warnings.length + ' warning' +
            (pendingAutofixData.warnings.length !== 1 ? 's' : '') + ' require manual review.';
    }
    showAutofixBanner(msg, 'success');

    // Show undo button
    document.getElementById('undo-autofix-btn').hidden = false;
}

function undoAutofix() {
    if (!preFixSnapshot) return;

    // Restore the table from snapshot
    renderEditTable(preFixSnapshot);

    // Re-highlight any original errors if we still have them
    preFixSnapshot = null;
    pendingAutofixData = null;
    document.getElementById('undo-autofix-btn').hidden = true;
    showAutofixBanner('Fixes undone. Table restored to pre-fix state.', 'info');
}

function closeAutofixPreview() {
    document.getElementById('autofix-preview').hidden = true;
}

function showAutofixBanner(message, type) {
    const banner = document.getElementById('autofix-banner');
    banner.hidden = false;
    banner.className = 'autofix-banner autofix-' + type;
    banner.innerHTML = '';
    const text = document.createElement('span');
    text.textContent = message;
    banner.appendChild(text);
    const dismiss = document.createElement('button');
    dismiss.className = 'autofix-dismiss';
    dismiss.textContent = '\u00d7';
    dismiss.addEventListener('click', () => { banner.hidden = true; });
    banner.appendChild(dismiss);
}

function escHtml(s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
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

function scrollToTop() {
    window.scrollTo({ top: 0, behavior: 'smooth' });
}

function startOver() {
    const onDashboard = !document.getElementById('dashboard').hidden;
    const onEditor = !document.getElementById('error-editor').hidden;

    if (onEditor && !confirm('Discard all edits and start over with a new file?')) return;

    document.getElementById('error-editor').hidden = true;
    document.getElementById('dashboard').hidden = true;
    document.getElementById('upload-section').hidden = false;
    document.getElementById('btn-new-analysis').classList.remove('visible');
    dropZone.style.display = '';
    fileInput.value = '';
    currentId = null;

    // Reset charts
    if (speciesChart) { speciesChart.destroy(); speciesChart = null; }
    if (diameterChart) { diameterChart.destroy(); diameterChart = null; }
    if (growthChart) { growthChart.destroy(); growthChart = null; }
}

// Expose to HTML onclick handlers
window.runGrowth = runGrowth;
window.exportData = exportData;
window.autofixData = autofixData;
window.revalidateData = revalidateData;
window.startOver = startOver;
window.loadStatistics = loadStatistics;
window.autofixSelectByConfidence = autofixSelectByConfidence;
window.applySelectedFixes = applySelectedFixes;
window.closeAutofixPreview = closeAutofixPreview;
window.undoAutofix = undoAutofix;
window.scrollToTop = scrollToTop;
