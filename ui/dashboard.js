const $ = id => document.getElementById(id);
const invoke = window.__TAURI__?.core?.invoke;
const PAGE_SIZE = 12;
let samples = [], blocks = [], blocksOldestFirst = [], report = { work_seconds: 0, idle_seconds: 0, projects: [] }, rules = [], paused = false, pageIndex = 0, selectedProject = '', generation = 0, settingsDirty = false;
let settings = { interval_minutes: 5, idle_minutes: 5, excluded_apps: [] };
const localDate = d => `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;
const time = v => new Date(v).toLocaleTimeString('ja-JP',{hour:'2-digit',minute:'2-digit'});
const el = (tag,value='',className='') => { const n=document.createElement(tag); n.textContent=value; if(className)n.className=className; return n; };
$('date').value=localDate(new Date());

function showError(message){ $('error').textContent=String(message||''); $('error').hidden=!message; }
function emptyState(message){
  const wrap=el('div','','empty-state'), icon=document.createElementNS('http://www.w3.org/2000/svg','svg');
  icon.setAttribute('viewBox','0 0 24 24'); const path=document.createElementNS(icon.namespaceURI,'path');
  path.setAttribute('d','M5 6h14v13H5zM8 3v6m8-6v6M8 13h8'); icon.append(path); wrap.append(icon,el('p',message)); return wrap;
}
function formatDuration(seconds){
  const minutes=Math.max(0,Math.round(seconds/60));
  const hours=Math.floor(minutes/60), rest=minutes%60;
  return hours?`${hours}h ${String(rest).padStart(2,'0')}m`:`${minutes}m`;
}
function timeRange(block){return `${time(block.start)}〜${time(block.end)}`;}
function filteredRows(){
  const query=$('search').value.trim().toLocaleLowerCase(), app=$('app-filter').value;
  return blocks.filter(b=>(!app||b.app===app)&&(!selectedProject||(b.project||'未分類')===selectedProject)&&`${b.app} ${b.title} ${b.project||''}`.toLocaleLowerCase().includes(query));
}
function renderAppFilter(){
  const current=$('app-filter').value, names=[...new Set(blocks.filter(b=>b.app).map(b=>b.app))].sort((a,b)=>a.localeCompare(b,'ja'));
  $('app-filter').replaceChildren(new Option('すべてのアプリ',''),...names.map(n=>new Option(n,n))); if(names.includes(current))$('app-filter').value=current;
}
function renderProjects(work){
  const groups=new Map(); for(const s of work)groups.set(s.project||'未分類',(groups.get(s.project||'未分類')||0)+1);
  const max=Math.max(...groups.values(),1);
  const nodes=[...groups].sort((a,b)=>b[1]-a[1]).slice(0,8).map(([name,count])=>{
    const b=el('button','','project-item'); b.type='button'; b.setAttribute('aria-pressed',String(selectedProject===name)); b.setAttribute('aria-label',`${name}で絞り込む、${count}件`);
    const cap=el('span','','project-caption'); cap.append(el('span',name),el('span',String(count)));
    const bar=document.createElement('progress'); bar.max=max; bar.value=count; b.append(cap,bar);
    b.onclick=()=>{selectedProject=selectedProject===name?'':name;pageIndex=0;render();}; return b;
  });
  $('project-list').replaceChildren(...nodes); if(!nodes.length)$('project-list').append(el('p','この日のプロジェクトはまだありません。'));
}
function blockTitle(b){
  if(b.status==='idle')return '離席・操作なし';
  if(b.status==='permission_required')return 'アクセシビリティ権限が必要です';
  return b.title||'タイトルを取得できません';
}
function activityRow(b){
  const row=el('article','','activity-row'), clock=el('time',''); clock.dateTime=b.start;
  clock.append(el('span',timeRange(b),'time-range'),el('span',formatDuration(b.seconds),'duration-text'));
  const idle=b.status==='idle', detail=el('div'); detail.append(el('span',blockTitle(b),'title-text'),el('span',b.app||`${settings.idle_minutes}分以上操作なし`,'app-name'));
  const project=el('div','','project-cell'); project.append(el('span',idle?'—':b.project||'未分類','project-name'));
  if(!idle)project.append(el('span',b.source==='rule'?'分類ルール':b.project?'タイトルから推定':'プロジェクト不明','source-name'));
  row.append(clock,detail,project);
  if(idle)row.append(document.createElement('span')); else { const button=el('button','分類','classify-button quiet-button'); button.type='button'; button.setAttribute('aria-label',`${timeRange(b)}の記録を分類`); button.onclick=()=>classify(b); row.append(button); }
  return row;
}
function renderReport(){
  $('report-work').textContent=formatDuration(report.work_seconds);
  $('report-idle').textContent=formatDuration(report.idle_seconds);
  const max=Math.max(...report.projects.map(p=>p.seconds),1);
  const total=Math.max(report.work_seconds,1);
  const closed=new Set([...$('report-projects').querySelectorAll('details:not([open])')].map(d=>d.dataset.project));
  const nodes=report.projects.map(project=>{
    const name=project.project||'未分類';
    const item=el('details','','report-project'); item.dataset.project=name; item.open=!closed.has(name);
    const summary=el('summary','');
    const cap=el('span','','project-caption');
    cap.append(el('span',name),el('span',formatDuration(project.seconds)));
    const bar=document.createElement('progress'); bar.max=max; bar.value=project.seconds;
    summary.append(cap,bar,el('small',`${Math.round(project.seconds/total*100)}%`));
    const entries=el('div','','report-entries');
    for(const entry of project.entries){
      const row=el('div','','report-entry');
      row.append(el('span',`${entry.app} — ${entry.title||'タイトルを取得できません'}`),el('span',formatDuration(entry.seconds)));
      entries.append(row);
    }
    item.append(summary,entries);
    return item;
  });
  $('report-projects').replaceChildren(...nodes); if(!nodes.length)$('report-projects').append(el('p','この日の作業時間はまだありません。','muted'));
}
function render(){
  const work=samples.filter(s=>s.status!=='idle');
  $('count').textContent=work.length; $('project-count').textContent=new Set(work.map(s=>s.project).filter(Boolean)).size; $('app-count').textContent=new Set(work.map(s=>s.app).filter(Boolean)).size; $('interval-display').textContent=settings.interval_minutes;
  renderProjects(work); renderReport(); $('active-filter').hidden=!selectedProject; $('filter-label').textContent=selectedProject?`プロジェクト：${selectedProject}`:'';
  const filtered=filteredRows(), pageCount=Math.max(1,Math.ceil(filtered.length/PAGE_SIZE)); pageIndex=Math.min(pageIndex,pageCount-1);
  const visible=filtered.slice(pageIndex*PAGE_SIZE,(pageIndex+1)*PAGE_SIZE); $('entries').replaceChildren(...visible.map(activityRow));
  if(!visible.length)$('entries').append(emptyState($('search').value||$('app-filter').value||selectedProject?'条件に一致する記録がありません。':'この日の記録はまだありません。'));
  $('result-count').textContent=`${filtered.length}件`; $('page-number').textContent=`${pageIndex+1} / ${pageCount}`; $('previous-page').disabled=pageIndex===0; $('next-page').disabled=pageIndex>=pageCount-1; $('export').disabled=!blocksOldestFirst.length;
}
function renderRules(){
  $('rule-count').textContent=`${rules.length}件`;
  const nodes=rules.map(r=>{const row=el('div','','rule-item'),d=el('div',r.project);d.append(el('small',`${r.app} / 「${r.contains}」を含む`));const b=el('button','削除','quiet-button');b.type='button';b.onclick=async()=>{b.disabled=true;try{await invoke('delete_rule',{id:r.id});await refresh();}catch(e){showError(e);b.disabled=false;}};row.append(d,b);return row;});
  $('rules').replaceChildren(...nodes); if(!nodes.length)$('rules').append(el('p','分類ルールはまだありません。タイムラインの「分類」から追加できます。','muted'));
}
function fillSettings(){
  if(settingsDirty)return; $('interval').value=settings.interval_minutes; $('idle').value=settings.idle_minutes; $('excluded').value=settings.excluded_apps.join('\n');
  [$('interval'),$('idle'),$('excluded'),$('save-settings')].forEach(c=>c.disabled=false);
}
function updateStatus(state){
  paused=state.paused; $('status').dataset.state=paused?'paused':state.last_error?'error':'recording'; $('status').textContent=paused?'一時停止中':state.last_error?'取得を確認':`記録中 · ${settings.interval_minutes}分ごと`;
  $('pause').disabled=false;$('pause').textContent=paused?'記録を再開':'一時停止';$('last').textContent=state.last_capture?`最終記録 ${new Date(state.last_capture).toLocaleString('ja-JP')}`:'起動後、最初の記録を待っています。';
  $('next-capture').textContent=!paused&&state.next_capture?`次回 ${new Date(state.next_capture).toLocaleTimeString('ja-JP',{hour:'2-digit',minute:'2-digit'})} ごろ`:'';$('database').textContent=state.database;showError(state.last_error||'');
}
async function refresh(){
  if(!invoke){$('status').dataset.state='disconnected';$('status').textContent='未接続';showError('Worklogアプリから開いてください。ブラウザ単体では記録に接続できません。');$('entries').replaceChildren(emptyState('Worklogアプリの起動を待っています。'));return;}
  const current=++generation;
  try{const [rows,summary,state,currentRules,currentSettings]=await Promise.all([invoke('day',{date:$('date').value}),invoke('day_summary',{date:$('date').value}),invoke('status'),invoke('rules'),invoke('get_settings')]);if(current!==generation)return;samples=rows;blocksOldestFirst=summary.blocks;blocks=[...summary.blocks].reverse();report=summary.report;rules=currentRules;settings=currentSettings;updateStatus(state);fillSettings();renderAppFilter();render();renderRules();}
  catch(e){if(current===generation){showError(e);$('status').dataset.state='error';$('status').textContent='接続エラー';}}
}
async function loadAutostart(){
  if(!invoke)return;try{$('autostart').checked=await invoke('get_autostart');$('autostart').disabled=false;$('quit').disabled=false;}catch(e){$('autostart-message').textContent=String(e);$('autostart-message').dataset.error='true';}
}
async function loadAccessibility(){
  if(!invoke)return;
  try{
    const state=await invoke('accessibility_status');
    $('accessibility-row').hidden=!state.supported;
    if(!state.supported)return;
    $('accessibility-label').textContent=state.trusted?'アクセシビリティ：許可済み':'アクセシビリティ：許可が必要です';
    $('accessibility-message').textContent=state.trusted?'ウィンドウタイトルを取得できます。':'このWorklogアプリ本体に許可を付けてください。';
    $('accessibility-executable').textContent=state.executable;
    $('request-accessibility').textContent=state.trusted?'権限を再確認':'許可を設定';
    $('request-accessibility').disabled=false;
  }catch(e){
    $('accessibility-row').hidden=false;
    $('accessibility-label').textContent='アクセシビリティを確認できません';
    $('accessibility-message').textContent=String(e);
  }
}
function switchPage(){
  const page=location.hash==='#settings'?'settings':location.hash==='#report'?'report':'dashboard';$('dashboard-page').hidden=page!=='dashboard';$('report-page').hidden=page!=='report';$('settings-page').hidden=page!=='settings';$('shared-day-toolbar').hidden=page==='settings';$('page-title').textContent=page==='dashboard'?'ダッシュボード':page==='report'?'レポート':'設定';
  document.querySelectorAll('[data-page]').forEach(a=>a.dataset.page===page?a.setAttribute('aria-current','page'):a.removeAttribute('aria-current'));if(page==='settings'){loadAutostart();loadAccessibility();}
}
function moveDate(days){const d=new Date(`${$('date').value}T12:00:00`);d.setDate(d.getDate()+days);$('date').value=localDate(d);selectedProject='';pageIndex=0;refresh();}
function titleSegments(title){
  const seen=new Set(), parts=[];
  for(const part of String(title||'').split(/\s+(?:—|–|-|·|\|)\s+/)){
    const value=part.trim();
    if(value&&!seen.has(value)){seen.add(value);parts.push(value);}
  }
  return parts;
}
function matchingSamples(app,contains){
  const value=contains.trim();
  if(!value)return [];
  return samples.filter(s=>s.status!=='idle'&&s.app===app&&String(s.title||'').includes(value));
}
function classify(block){
  const f=$('rule-form'), contains=f.elements.contains, segments=titleSegments(block.title), segmentGroup=$('title-segments');
  f.elements.app.value=block.app;
  contains.value=segments.length>1?segments[segments.length-1]:block.title;
  f.elements.project.value=block.project||'';
  $('rule-error').textContent='';
  const updatePreview=()=>{
    const value=contains.value.trim();
    segmentGroup.querySelectorAll('button').forEach(b=>b.setAttribute('aria-pressed',String(b.dataset.value===value)));
    const matches=matchingSamples(block.app,value);
    $('classify-match').textContent=value?(matches.length?`表示中の日の記録 ${matches.length}件に一致します`:'表示中の日の記録には一致しません（ほかの日の記録には一致する場合があります）'):'';
    const examples=[];
    for(const match of matches){
      const title=match.title||'タイトルを取得できません';
      if(!examples.includes(title))examples.push(title);
      if(examples.length>=3)break;
    }
    $('classify-examples').replaceChildren(...(value&&examples.length?[el('span',`例：${examples.join(' / ')}`)]:[]));
  };
  segmentGroup.replaceChildren(...segments.map(segment=>{
    const button=el('button',segment);
    button.type='button';
    button.dataset.value=segment;
    button.onclick=()=>{contains.value=segment;updatePreview();contains.focus();};
    return button;
  }));
  segmentGroup.hidden=segments.length<2;
  contains.oninput=updatePreview;
  updatePreview();
  $('classify').showModal();
}

$('rule-form').onsubmit=async e=>{e.preventDefault();e.submitter.disabled=true;try{await invoke('add_rule',Object.fromEntries(new FormData(e.currentTarget)));$('classify').close();await refresh();}catch(x){$('rule-error').textContent=String(x);}finally{e.submitter.disabled=false;}};
$('cancel-rule').onclick=()=>$('classify').close(); $('classify').onclick=e=>{if(e.target===$('classify'))$('classify').close();}; $('memo').onclick=e=>{if(e.target===$('memo'))$('memo').close();};
$('previous-day').onclick=()=>moveDate(-1);$('next-day').onclick=()=>moveDate(1);$('today').onclick=()=>{$('date').value=localDate(new Date());selectedProject='';pageIndex=0;refresh();};
$('date').onchange=()=>{selectedProject='';pageIndex=0;refresh();};$('refresh').onclick=refresh;$('search').oninput=()=>{pageIndex=0;render();};$('app-filter').onchange=()=>{pageIndex=0;render();};$('clear-filter').onclick=()=>{selectedProject='';pageIndex=0;render();};$('previous-page').onclick=()=>{pageIndex--;render();};$('next-page').onclick=()=>{pageIndex++;render();};
$('pause').onclick=async()=>{$('pause').disabled=true;try{await invoke('set_paused',{paused:!paused});await refresh();}catch(e){showError(e);$('pause').disabled=false;}};
$('settings-form').oninput=()=>{settingsDirty=true;$('settings-message').textContent='';};
$('settings-form').onsubmit=async e=>{e.preventDefault();const b=$('save-settings');b.disabled=true;$('settings-message').textContent='保存中…';$('settings-message').dataset.error='false';const next={interval_minutes:Number($('interval').value),idle_minutes:Number($('idle').value),excluded_apps:$('excluded').value.split('\n').map(v=>v.trim()).filter(Boolean)};try{settings=await invoke('save_settings',{settings:next});settingsDirty=false;fillSettings();$('settings-message').textContent='保存しました。';await refresh();}catch(x){$('settings-message').textContent=String(x);$('settings-message').dataset.error='true';}finally{b.disabled=false;}};
$('autostart').onchange=async()=>{const enabled=$('autostart').checked;$('autostart').disabled=true;$('autostart-message').textContent='変更中…';$('autostart-message').dataset.error='false';try{await invoke('set_autostart',{enabled});$('autostart-message').textContent=enabled?'次回のログインから自動で起動します。':'自動起動を解除しました。';}catch(e){$('autostart').checked=!enabled;$('autostart-message').textContent=String(e);$('autostart-message').dataset.error='true';}finally{$('autostart').disabled=false;}};
$('request-accessibility').onclick=async()=>{$('request-accessibility').disabled=true;try{await invoke('request_accessibility_permission');$('accessibility-message').textContent='システム設定でWorklogを有効にし、アプリを再起動してください。';setTimeout(loadAccessibility,1500);}catch(e){$('accessibility-message').textContent=String(e);$('request-accessibility').disabled=false;}};
$('export').onclick=()=>{$('memo-text').value=`${$('date').value} 稼働メモ\n※記録間隔から推定した時間です。記録と記録の間の作業は含まれません。\n\n${blocksOldestFirst.map(b=>`${timeRange(b)} (${formatDuration(b.seconds)})  ${b.status==='idle'?'離席・操作なし':`[${b.project||'未分類'}] ${b.app} — ${blockTitle(b)}`}`).join('\n')}`;$('copy-status').textContent='';$('memo').showModal();};
$('close-memo').onclick=()=>$('memo').close();$('copy').onclick=async()=>{try{await navigator.clipboard.writeText($('memo-text').value);$('copy-status').textContent='コピーしました。';}catch{$('memo-text').select();$('copy-status').textContent='⌘C または Ctrl+C でコピーしてください。';}};
$('quit').onclick=async()=>{$('quit').disabled=true;await invoke('quit_app');};window.addEventListener('hashchange',switchPage);switchPage();refresh();setInterval(()=>{if(!$('classify').open&&!$('memo').open&&!settingsDirty)refresh();},15000);
