const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const uiRoot = path.resolve(__dirname, '../ui');
const screenshotDir = process.env.UI_SCREENSHOT_DIR || path.join(os.tmpdir(), 'worklog-ui-test');
fs.mkdirSync(screenshotDir, { recursive: true });
(async()=>{
 const browser = await chromium.launch({headless:true, ...(process.env.CHROME_PATH ? {executablePath:process.env.CHROME_PATH} : {})});
 try {
 const page = await browser.newPage();
 await page.route('http://worklog.test/**', async route => {const name=new URL(route.request().url()).pathname.slice(1)||'index.html';const body=fs.readFileSync(path.join(uiRoot, name));await route.fulfill({body,contentType:name.endsWith('.css')?'text/css':name.endsWith('.js')?'text/javascript':'text/html'});});
 const errors=[]; page.on('pageerror',e=>errors.push(String(e)));
 await page.addInitScript(()=>{
  let paused=false,rules=[],autostart=false,settings={interval_minutes:5,idle_minutes:5,excluded_apps:[]}; window.calls=[];
  const rows=[
   {timestamp:'2026-09-13T08:55:00+09:00',interval_minutes:5,app:'システム設定',title:'',project:null,status:'title_unavailable',source:'none'},
   {timestamp:'2026-09-13T09:00:00+09:00',interval_minutes:5,app:'Code',title:'main.rs — worklog — Visual Studio Code',project:'worklog',status:'active',source:'title'},
   {timestamp:'2026-09-13T09:05:00+09:00',interval_minutes:5,app:'Code',title:'main.rs — worklog — Visual Studio Code',project:'worklog',status:'active',source:'title'},
   {timestamp:'2026-09-13T09:30:00+09:00',interval_minutes:5,app:'Firefox',title:'<img src=x onerror="window.INJECTED=1"> ChatGPT — 設計の相談',project:null,status:'active',source:'title'},
   {timestamp:'2026-09-13T09:35:00+09:00',interval_minutes:5,app:'',title:'',project:null,status:'idle',source:'idle'},
   {timestamp:'2026-09-13T09:40:00+09:00',interval_minutes:5,app:'Terminal',title:'Worklog — プロジェクトの作業',project:'Project'+ 'VeryLongName'.repeat(15),status:'active',source:'title'}
  ];
  const summary={
   blocks:[
    {start:'2026-09-13T08:55:00+09:00',end:'2026-09-13T09:00:00+09:00',seconds:300,app:'システム設定',title:'',project:null,status:'title_unavailable',source:'none',sample_count:1},
    {start:'2026-09-13T09:00:00+09:00',end:'2026-09-13T09:30:00+09:00',seconds:1800,app:'Code',title:'main.rs — worklog — Visual Studio Code',project:'worklog',status:'active',source:'title',sample_count:2},
    {start:'2026-09-13T09:30:00+09:00',end:'2026-09-13T09:35:00+09:00',seconds:300,app:'Firefox',title:'<img src=x onerror="window.INJECTED=1"> ChatGPT — 設計の相談',project:null,status:'active',source:'title',sample_count:1},
    {start:'2026-09-13T09:35:00+09:00',end:'2026-09-13T09:40:00+09:00',seconds:300,app:'',title:'',project:null,status:'idle',source:'idle',sample_count:1},
    {start:'2026-09-13T09:40:00+09:00',end:'2026-09-13T09:45:00+09:00',seconds:300,app:'Terminal',title:'Worklog — プロジェクトの作業',project:'Project'+ 'VeryLongName'.repeat(15),status:'active',source:'title',sample_count:1}
   ],
   report:{work_seconds:2700,idle_seconds:300,projects:[
    {project:'worklog',seconds:1800,entries:[{app:'Code',title:'main.rs — worklog — Visual Studio Code',seconds:1800}]},
    {project:null,seconds:600,entries:[{app:'システム設定',title:'',seconds:300},{app:'Firefox',title:'<img src=x onerror="window.INJECTED=1"> ChatGPT — 設計の相談',seconds:300}]},
    {project:'Project'+ 'VeryLongName'.repeat(15),seconds:300,entries:[{app:'Terminal',title:'Worklog — プロジェクトの作業',seconds:300}]}
   ]}
  };
  window.__TAURI__={core:{invoke:async(cmd,args)=>{window.calls.push({cmd,args}); if(cmd==='day')return rows;if(cmd==='day_summary')return summary;if(cmd==='status')return{paused,last_capture:'2026-09-13T09:40:00+09:00',next_capture:'2026-09-13T09:45:00+09:00',last_error:null,database:'/Users/example/Library/Application Support/jp.worklog/worklog.sqlite'};if(cmd==='rules')return rules;if(cmd==='get_settings')return settings;if(cmd==='save_settings'){settings=args.settings;return settings;}if(cmd==='get_autostart')return autostart;if(cmd==='set_autostart'){autostart=args.enabled;return;}if(cmd==='accessibility_status'||cmd==='request_accessibility_permission')return{supported:true,trusted:false,executable:'/Applications/Worklog.app/Contents/MacOS/worklog'};if(cmd==='quit_app')return;if(cmd==='set_paused'){paused=args.paused;return;}if(cmd==='add_rule'){rules.push({id:1,...args});return;}if(cmd==='delete_rule'){rules=[];return;}throw Error(cmd);}}};
 });
 await page.goto('http://worklog.test/index.html');
 await page.waitForFunction(()=>document.querySelector('#count').textContent==='5');
 assert.equal(await page.locator('#entries article').count(),5);
 assert.match(await page.locator('#entries article').first().textContent(),/Terminal/);
 assert.equal(await page.locator('#entries article').filter({hasText:'システム設定'}).locator('button').count(),0);
 assert.match(await page.locator('#entries').textContent(),/30m/);
 assert.match(await page.locator('#entries').textContent(),/〜/);
 assert.equal(await page.locator('#entries img').count(),0);
 assert.equal(await page.evaluate(()=>window.INJECTED),undefined);
 const layouts=[];
 for(const width of [320,375,414,768,1120]){
  await page.setViewportSize({width,height:900});
  const overflow=await page.evaluate(()=>[...document.querySelectorAll('body *')].filter(e=>e.getClientRects().length && !e.closest('dialog') && !e.classList.contains('sr-only')).map(e=>({tag:e.tagName,id:e.id,cls:e.className,x:e.getBoundingClientRect().x,right:e.getBoundingClientRect().right,scroll:e.scrollWidth,client:e.clientWidth})).filter(e=>e.x<-.5||e.right>innerWidth+.5));
  assert.deepEqual(overflow,[],`overflow at ${width}: ${JSON.stringify(overflow)}`);
  layouts.push({width,overflow});
  await page.screenshot({path:path.join(screenshotDir, `worklog-${width}.png`),fullPage:true});
 }
 await page.locator('[data-page=report]').click();await page.waitForFunction(()=>!document.querySelector('#report-page').hidden);
 assert.match(await page.locator('#report-projects').textContent(),/worklog/);assert.match(await page.locator('#report-projects').textContent(),/30m/);
 for(const width of [320,375,414,768,1120]){
  await page.setViewportSize({width,height:900});
  const overflow=await page.evaluate(()=>[...document.querySelectorAll('body *')].filter(e=>e.getClientRects().length && !e.closest('dialog') && !e.classList.contains('sr-only')).map(e=>({tag:e.tagName,id:e.id,cls:e.className,x:e.getBoundingClientRect().x,right:e.getBoundingClientRect().right,scroll:e.scrollWidth,client:e.clientWidth})).filter(e=>e.x<-.5||e.right>innerWidth+.5));
  assert.deepEqual(overflow,[],`report overflow at ${width}: ${JSON.stringify(overflow)}`);
 }
 await page.locator('[data-page=dashboard]').click();await page.waitForFunction(()=>!document.querySelector('#dashboard-page').hidden);
 await page.locator('#search').fill('ChatGPT');assert.equal(await page.locator('#entries article').count(),1);
 await page.locator('.activity-row button').first().click();assert.equal(await page.locator('#classify').evaluate(e=>e.open),true);
 assert.equal(await page.locator('#title-segments img, #classify-examples img').count(),0);
 await page.locator('#cancel-rule').click();await page.waitForFunction(()=>!document.querySelector('#classify').open);
 await page.locator('#search').fill('');
 await page.locator('.activity-row button').first().click();assert.equal(await page.locator('#classify').evaluate(e=>e.open),true);
 assert.equal(await page.locator('[name=contains]').inputValue(),'プロジェクトの作業');
 assert.deepEqual(await page.locator('#title-segments button').evaluateAll(buttons=>buttons.map(b=>b.textContent)),['Worklog','プロジェクトの作業']);
 await page.locator('#title-segments button').filter({hasText:'Worklog'}).click();
 assert.equal(await page.locator('[name=contains]').inputValue(),'Worklog');
 assert.equal(await page.locator('#title-segments button').filter({hasText:'Worklog'}).getAttribute('aria-pressed'),'true');
 assert.match(await page.locator('#classify-match').textContent(),/1件/);
 await page.locator('[name=contains]').fill('存在しない文字列');assert.match(await page.locator('#classify-match').textContent(),/一致しません/);
 await page.locator('#title-segments button').filter({hasText:'Worklog'}).click();
 await page.locator('[name=project]').fill('Review project');await page.locator('#rule-form button[type=submit]').click();await page.waitForFunction(()=>!document.querySelector('#classify').open);assert.match(await page.locator('#rules').textContent(),/Review project/);
 await page.locator('.activity-row button').first().click();assert.equal(await page.locator('#classify').evaluate(e=>e.open),true);
 await page.locator('#title-segments button').filter({hasText:'Worklog'}).click();
 assert.match(await page.locator('#classify-warning').textContent(),/同じ条件のルールがすでにあります/);
 await page.locator('[name=contains]').fill('プロジェクトの作業');
 assert.match(await page.locator('#classify-warning').textContent(),/既存のルール「Worklog」/);
 await page.locator('#cancel-rule').click();await page.waitForFunction(()=>!document.querySelector('#classify').open);
 await page.locator('#pause').click();await page.waitForFunction(()=>document.querySelector('#status').textContent==='一時停止中');await page.locator('#pause').click();await page.waitForFunction(()=>document.querySelector('#status').textContent==='記録中 · 5分ごと');
 await page.locator('[data-page=settings]').click();await page.waitForFunction(()=>!document.querySelector('#settings-page').hidden);
 await page.waitForFunction(()=>document.querySelector('#accessibility-executable').textContent.includes('/Applications/Worklog.app'));
 assert.match(await page.locator('#accessibility-label').textContent(),/許可が必要/);await page.locator('#request-accessibility').click();assert.equal(await page.evaluate(()=>window.calls.some(c=>c.cmd==='request_accessibility_permission')),true);
 await page.locator('#interval').fill('10');await page.locator('#idle').fill('8');await page.locator('#excluded').fill('1Password\nMessages');await page.locator('#save-settings').click();await page.waitForFunction(()=>document.querySelector('#settings-message').textContent==='保存しました。');
 assert.equal(await page.evaluate(()=>window.calls.some(c=>c.cmd==='save_settings'&&c.args.settings.interval_minutes===10)),true);
 await page.locator('#autostart').check();await page.waitForFunction(()=>document.querySelector('#autostart-message').textContent.includes('自動で起動'));
 await page.locator('[data-page=dashboard]').click();await page.waitForFunction(()=>!document.querySelector('#dashboard-page').hidden);
 await page.locator('#export').click();const memo=await page.locator('#memo-text').inputValue();assert.match(memo,/ChatGPT/);assert.match(memo,/離席・操作なし/);assert.ok(memo.indexOf('Code')<memo.indexOf('Terminal'));await page.locator('#close-memo').click();
 assert.deepEqual(errors,[]);console.log(JSON.stringify({passed:true,layouts,checks:['unsafe title rendered as text','newest timeline first','duration and range','title unavailable cannot classify','report page','search','classification save','duplicate classification warning','overlap classification warning','pause/resume','accessibility permission','settings save','autostart','export oldest first','no runtime errors'],screenshots:path.join(screenshotDir, 'worklog-{width}.png')},null,2));
 } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exit(1)});
