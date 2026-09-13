const $ = (id) => document.getElementById(id);
const invoke = window.__TAURI__?.core?.invoke;
const localDate = (date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
const time = (date) => new Date(date).toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit' });
let samples = [], paused = false, generation = 0;
$('date').value = localDate(new Date());
function element(tag, text, className) {
  const node = document.createElement(tag); node.textContent = text;
  if (className) node.className = className;
  return node;
}
function error(message) { $('error').textContent = String(message); $('error').hidden = !message; }
function render() {
  const work = samples.filter(s => s.status !== 'idle');
  $('count').textContent = work.length;
  const groups = new Map();
  for (const s of work) groups.set(s.project || '未分類', (groups.get(s.project || '未分類') || 0) + 1);
  $('project-count').textContent = [...groups.keys()].filter(p => p !== '未分類').length;
  $('project-list').replaceChildren(...[...groups].sort((a,b) => b[1]-a[1]).map(([name,count]) => {
    const row = element('div', '', 'project-row'); row.append(element('span', name), element('span', `${count} 回の記録`)); return row;
  }));
  if (!groups.size) $('project-list').append(element('p', 'この日のプロジェクトはまだありません。', 'muted'));
  const query = $('search').value.toLocaleLowerCase();
  const filtered = samples.filter(s => `${s.app} ${s.title} ${s.project || ''}`.toLocaleLowerCase().includes(query));
  $('entries').replaceChildren(...filtered.map(s => {
    const row = element('article', '', 'entry');
    const clock = element('time', time(s.timestamp)); clock.dateTime = s.timestamp;
    const content = element('div', '');
    content.append(element('h3', s.status === 'idle' ? '離席・操作なし' : s.title || 'タイトルを取得できませんでした'));
    const meta = element('p', s.app || '5分以上操作がありません');
    if (s.status !== 'idle') {
      const source = s.source === 'rule' ? '分類ルール' : s.project ? 'タイトルから推定' : 'プロジェクト不明';
      meta.append(element('span', ` · ${s.project || '未分類'} · ${source}`, s.project ? 'project-tag' : ''));
    }
    content.append(meta); row.append(clock, content);
    if (s.status !== 'idle') { const button = element('button', '分類'); button.setAttribute('aria-label', `${time(s.timestamp)}の記録を分類`); button.onclick = () => classify(s); row.append(button); }
    return row;
  }));
  if (!filtered.length) $('entries').append(element('p', samples.length ? '条件に一致する記録がありません。' : 'この日の記録はまだありません。アプリを起動したまま作業すると、ここに記録がたまります。', 'empty'));
  $('export').disabled = !samples.length;
}
async function refresh() {
  if (!invoke) { error('Tauriアプリから開いてください。ブラウザ単体では記録データに接続できません。'); $('status').textContent = '未接続'; $('entries').replaceChildren(element('p', 'Tauriアプリの起動を待っています。', 'empty')); return; }
  const current = ++generation;
  try {
    const [rows, state, rules] = await Promise.all([invoke('day', { date: $('date').value }), invoke('status'), invoke('rules')]);
    if (current !== generation) return;
    samples = rows; paused = state.paused;
    $('status').textContent = paused ? '一時停止中' : state.last_error ? '取得を確認' : '記録中 · 5分ごと';
    $('pause').disabled = false; $('pause').textContent = paused ? '記録を再開' : '一時停止';
    $('last').textContent = state.last_capture ? `最終記録 ${new Date(state.last_capture).toLocaleString('ja-JP')}` : '起動から約10秒後に最初の記録を取得します。';
    $('database').textContent = state.database;
    error(state.last_error || ''); render(); renderRules(rules);
  } catch (e) { if (current === generation) { error(e); $('status').textContent = '接続エラー'; } }
}
function renderRules(rules) {
  $('rules').replaceChildren(...rules.map(r => {
    const row = element('div', '', 'rule'), text = element('div', r.project);
    text.append(element('small', `${r.app} / 「${r.contains}」を含む`));
    const button = element('button', '削除'); button.onclick = async () => { button.disabled = true; try { await invoke('delete_rule', { id: r.id }); await refresh(); } catch (e) { error(e); button.disabled = false; } };
    row.append(text, button); return row;
  }));
  if (!rules.length) $('rules').append(element('p', '分類ルールはまだありません。記録の「分類」から追加できます。', 'muted'));
}
function classify(sample) {
  const form = $('rule-form'); form.elements.app.value = sample.app; form.elements.contains.value = sample.title;
  form.elements.project.value = sample.project || ''; $('rule-error').textContent = ''; $('classify').showModal();
}
$('rule-form').onsubmit = async event => {
  event.preventDefault(); const button = event.submitter; button.disabled = true;
  try { const data = Object.fromEntries(new FormData(event.target)); await invoke('add_rule', data); $('classify').close(); await refresh(); }
  catch (e) { $('rule-error').textContent = String(e); } finally { button.disabled = false; }
};
$('cancel-rule').onclick = () => $('classify').close();
$('refresh').onclick = refresh; $('date').onchange = refresh; $('search').oninput = render;
$('pause').onclick = async () => { $('pause').disabled = true; try { await invoke('set_paused', { paused: !paused }); await refresh(); } catch (e) { error(e); $('pause').disabled = false; } };
$('export').onclick = () => {
  $('memo-text').value = `${$('date').value} 稼働メモ\n※5分ごとの観測記録。実働時間は別途確認。\n\n` + samples.map(s => `${time(s.timestamp)}  ${s.status === 'idle' ? '離席・操作なし' : `[${s.project || '未分類'}] ${s.app} — ${s.title || 'タイトル不明'}`}`).join('\n');
  $('copy-status').textContent = ''; $('memo').showModal();
};
$('close-memo').onclick = () => $('memo').close();
$('copy').onclick = async () => {
  try { await navigator.clipboard.writeText($('memo-text').value); $('copy-status').textContent = 'コピーしました。'; }
  catch { $('memo-text').select(); $('copy-status').textContent = '自動コピーできませんでした。⌘Cでコピーしてください。'; }
};
refresh();
setInterval(() => { if (!$('classify').open && !$('memo').open) refresh(); }, 15000);
