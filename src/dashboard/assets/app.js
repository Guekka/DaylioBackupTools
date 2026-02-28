'use strict';
/* global vegaEmbed */
(function () {
    const RAW_DATA = JSON.parse(document.getElementById('diary-data').textContent);
    const el = id => document.getElementById(id);
    const fmt = x => x == null ? '-' : (typeof x === 'number' ? Number(x).toFixed(2) : x);

    // Controls
    const allDates = RAW_DATA.stats.mood.daily.map(d => d.date).sort();
    const minDate = allDates[0];
    const maxDate = allDates[allDates.length - 1];
    el('controls').innerHTML = [
        "<label>From<input type='date' id='filter-from' value='" + minDate + "'></label>",
        "<label>To<input type='date' id='filter-to' value='" + maxDate + "'></label>",
        "<label>Tag Filter<select id='filter-tags' multiple size='6'></select></label>",
        "<button id='apply-filters' type='button'>Apply Filters</button>"
    ].join('');
    const tagSelect = el('filter-tags');
    (RAW_DATA.tags || []).forEach(t => {
        const o = document.createElement('option');
        o.value = t.name;
        o.textContent = t.name;
        tagSelect.appendChild(o);
    });

    // KPI summary
    const kpis = el('kpis');
    const avg = RAW_DATA.stats.mood.average;
    const prev = RAW_DATA.stats.mood.previous_period_average;
    const delta = (avg != null && prev != null) ? (avg - prev) : null;
    kpis.innerHTML = [
        "<div class='kpi'><strong>Average mood</strong><br>" + fmt(avg) + (delta != null ? " <span class='small-note'>(Δ " + fmt(delta) + ")</span>" : '') + "</div>",
        "<div class='kpi'><strong>Entries</strong><br>" + RAW_DATA.metadata.total_entries + "</div>",
        "<div class='kpi'><strong>Days logged</strong><br>" + RAW_DATA.metadata.total_days_logged + "</div>",
        "<div class='kpi'><strong>Words total</strong><br>" + RAW_DATA.metadata.word_total + "</div>"
    ].join('');

    // Highlights
    const highlights = el('highlights');
    if ((RAW_DATA.highlights || []).length) {
        const items = RAW_DATA.highlights.map(h => "<li><strong>" + h.kind + ":</strong> " + h.message + "</li>").join('');
        highlights.innerHTML = "<h2>Highlights</h2><ul>" + items + "</ul>";
    } else {
        highlights.innerHTML = '<h2>Highlights</h2><p>No highlights.</p>';
    }

    function vl(spec, target) {
        vegaEmbed(target, spec, {actions: false});
    }

    function dateFilter(from, to, d) {
        return (!from || d >= from) && (!to || d <= to);
    }

    function getSelectedTags() {
        return Array.from(tagSelect.selectedOptions).map(o => o.value);
    }

    function sliceData() {
        const from = el('filter-from').value;
        const to = el('filter-to').value;
        const tagsSel = new Set(getSelectedTags());
        const dailyMood = RAW_DATA.stats.mood.daily.filter(d => dateFilter(from, to, d.date));
        const wordsDaily = RAW_DATA.stats.writing.words_daily.filter(d => dateFilter(from, to, d.date));
        const calendarDays = RAW_DATA.stats.calendar.days.filter(d => dateFilter(from, to, d.date));
        let tagUsage = RAW_DATA.stats.tags.usage.filter(u => dateFilter(from, to, u.first) || dateFilter(from, to, u.last));
        if (tagsSel.size) {
            tagUsage = tagUsage.filter(u => tagsSel.has(u.tag));
        }
        let tagPairs = RAW_DATA.stats.tags.pairs.filter(p => !tagsSel.size || (tagsSel.has(p.tags[0]) && tagsSel.has(p.tags[1])));
        let emerging = RAW_DATA.stats.tags.emerging.filter(e => !tagsSel.size || tagsSel.has(e.tag));
        let combos = RAW_DATA.stats.mood.combos;
        return {dailyMood, wordsDaily, calendarDays, tagUsage, tagPairs, emerging, combos};
    }

    function renderAll() {
        const S = sliceData();
        // Mood Over Time
        const moodValues = S.dailyMood.filter(d => d.avg != null);
        el('mood-time').innerHTML = "<div class='chart-block'><h2>Mood Over Time</h2><div id='mood-chart'></div></div>";
        if (moodValues.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: moodValues},
                mark: {type: 'line', point: true},
                width: 700,
                height: 240,
                encoding: {
                    x: {field: 'date', type: 'temporal'},
                    y: {field: 'avg', type: 'quantitative', title: 'Avg Mood'}
                }
            }, '#mood-chart');
        } else {
            el('mood-chart').innerHTML = '<p>No numeric mood data.</p>';
        }

        // Words Per Day
        el('writing').innerHTML = "<div class='chart-block'><h2>Words Per Day</h2><div id='words-chart'></div></div>";
        if (S.wordsDaily.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: S.wordsDaily},
                mark: 'bar',
                width: 700,
                height: 240,
                encoding: {x: {field: 'date', type: 'temporal'}, y: {field: 'words', type: 'quantitative'}}
            }, '#words-chart');
        } else {
            el('words-chart').innerHTML = '<p>No data</p>';
        }

        // Weekly Words
        el('weekly-words').innerHTML = "<div class='chart-block'><h2>Words Per Week</h2><div id='weekly-chart'></div></div>";
        const weeklyData = (RAW_DATA.stats.writing.weekly_words || []).slice();
        if (weeklyData.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: weeklyData},
                mark: {type: 'line', point: true},
                width: 700,
                height: 200,
                encoding: {
                    x: {field: 'week', type: 'ordinal', title: 'Week'},
                    y: {field: 'words', type: 'quantitative', title: 'Words'}
                }
            }, '#weekly-chart');
        } else {
            el('weekly-chart').innerHTML = '<p>No weekly data</p>';
        }

        // Monthly Words
        el('monthly-words').innerHTML = "<div class='chart-block'><h2>Words Per Month</h2><div id='monthly-chart'></div></div>";
        const monthlyData = (RAW_DATA.stats.writing.monthly_words || []).slice();
        if (monthlyData.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: monthlyData},
                mark: 'bar',
                width: 700,
                height: 200,
                encoding: {
                    x: {field: 'month', type: 'ordinal', title: 'Month'},
                    y: {field: 'words', type: 'quantitative', title: 'Words'}
                }
            }, '#monthly-chart');
        } else {
            el('monthly-chart').innerHTML = '<p>No monthly data</p>';
        }

        // Mood Distribution
        el('mood-distribution').innerHTML = "<div class='chart-block'><h2>Mood Distribution</h2><div id='mood-dist-chart'></div></div>";
        const distValues = RAW_DATA.stats.mood.distribution.map(d => ({mood: d.mood, count: d.count}));
        if (distValues.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: distValues},
                mark: 'bar',
                width: 700,
                height: 260,
                encoding: {x: {field: 'mood', type: 'nominal', sort: '-y'}, y: {field: 'count', type: 'quantitative'}}
            }, '#mood-dist-chart');
        }

        // Entry Length Histogram
        el('length-hist').innerHTML = "<div class='chart-block'><h2>Entry Length Histogram</h2><div id='length-chart'></div></div>";
        vl({
            $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
            data: {values: RAW_DATA.stats.writing.length_hist},
            mark: 'bar',
            width: 700,
            height: 240,
            encoding: {x: {field: 'bucket', type: 'ordinal'}, y: {field: 'count', type: 'quantitative'}}
        }, '#length-chart');

        // Weekday Mood
        el('weekday-mood').innerHTML = "<div class='chart-block'><h2>Weekday Mood</h2><div id='weekday-mood-chart'></div></div>";
        const wMood = RAW_DATA.stats.temporal.weekday_mood.filter(d => d.avg != null).map(d => ({
            weekday: d.weekday,
            avg: d.avg
        }));
        if (wMood.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: wMood},
                mark: 'bar',
                width: 700,
                height: 200,
                encoding: {x: {field: 'weekday', type: 'ordinal'}, y: {field: 'avg', type: 'quantitative'}}
            }, '#weekday-mood-chart');
        }

        // Hour Entries
        el('hour-entries').innerHTML = "<div class='chart-block'><h2>Hour Entries</h2><div id='hour-entries-chart'></div></div>";
        vl({
            $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
            data: {values: RAW_DATA.stats.temporal.hour_entries},
            mark: 'bar',
            width: 700,
            height: 200,
            encoding: {x: {field: 'hour', type: 'ordinal'}, y: {field: 'entries', type: 'quantitative'}}
        }, '#hour-entries-chart');

        // Mood Combos
        el('mood-combos').innerHTML = "<div class='chart-block'><h2>Mood Combos</h2><div id='mood-combos-chart'></div></div>";
        if (S.combos.length) {
            const comboValues = S.combos.map(c => ({combo: c.moods.join(' + '), count: c.count}));
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: comboValues},
                mark: 'bar',
                width: 700,
                height: 260,
                encoding: {y: {field: 'combo', type: 'nominal', sort: '-x'}, x: {field: 'count', type: 'quantitative'}}
            }, '#mood-combos-chart');
        }

        // Tag Usage
        el('tag-usage').innerHTML = "<div class='chart-block'><h2>Tag Usage</h2><div id='tag-usage-chart'></div></div>";
        if (S.tagUsage.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: S.tagUsage},
                mark: 'bar',
                width: 700,
                height: Math.min(600, 22 * S.tagUsage.length),
                encoding: {y: {field: 'tag', type: 'nominal', sort: '-x'}, x: {field: 'count', type: 'quantitative'}}
            }, '#tag-usage-chart');
        }

        // Tag Pairs
        el('tag-pairs').innerHTML = "<div class='chart-block'><h2>Tag Pairs</h2><div id='tag-pairs-chart'></div></div>";
        if (S.tagPairs.length) {
            const pairValues = S.tagPairs.map(p => ({pair: p.tags.join(' + '), count: p.count}));
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: pairValues},
                mark: 'bar',
                width: 700,
                height: Math.min(600, 22 * pairValues.length),
                encoding: {y: {field: 'pair', type: 'nominal', sort: '-x'}, x: {field: 'count', type: 'quantitative'}}
            }, '#tag-pairs-chart');
        }

        // Emerging Tags
        el('emerging-tags').innerHTML = "<div class='chart-block'><h2>Emerging Tags</h2><div id='emerging-chart'></div></div>";
        if (S.emerging.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: S.emerging},
                mark: 'bar',
                width: 700,
                height: 240,
                encoding: {
                    x: {field: 'tag', type: 'nominal', sort: '-y'},
                    y: {field: 'growth_factor', type: 'quantitative', title: 'Growth x'}
                },
                tooltip: [{field: 'previous_count'}, {field: 'current_count'}]
            }, '#emerging-chart');
        }

        // Calendar Heatmap
        el('calendar-heatmap').innerHTML = "<div class='chart-block'><h2>Calendar Heatmap</h2><div id='calendar-chart'></div></div>";
        if (S.calendarDays.length) {
            vl({
                $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
                data: {values: S.calendarDays},
                mark: 'rect',
                width: 700,
                height: 160,
                encoding: {
                    x: {field: 'date', type: 'temporal', timeUnit: 'day'},
                    y: {field: 'date', type: 'temporal', timeUnit: 'month'},
                    color: {field: 'mood_avg', type: 'quantitative', scale: {scheme: 'blues'}},
                    tooltip: [{field: 'date', type: 'temporal'}, {
                        field: 'mood_avg',
                        type: 'quantitative'
                    }, {field: 'entries', type: 'quantitative'}, {field: 'words', type: 'quantitative'}]
                }
            }, '#calendar-chart');
        }
    }

    renderAll();
    el('apply-filters').addEventListener('click', renderAll);
})();

