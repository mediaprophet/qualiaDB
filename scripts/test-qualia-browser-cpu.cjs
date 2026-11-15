// Actual browser CPU integration test. Requires built docs/playground artifacts, local
// SmolLM2 P64 model and installed Playwright. Set CHROMIUM_PATH, ARTIFACT_DIR,
// and NO_SAB=1 to select the browser, retained receipts, and transferable path.
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require('playwright');
const root=path.resolve(__dirname,'..');
const out=path.resolve(process.env.ARTIFACT_DIR || path.join(root,'target/qualia-browser-runtime'));
fs.mkdirSync(out,{recursive:true});
const crypto=require('node:crypto');
const hash=file=>crypto.createHash('sha256').update(fs.readFileSync(path.join(root,file))).digest('hex');
const canonical='coordination/worktrees/etalon-migration/crates/etalon-wasm/js/worker_transport.js';
const deployed='docs/js/etalon-worker-transport.js';
if(hash(canonical)!==hash(deployed))throw new Error('Canonical/deployed worker transport mismatch');
function writeReceipt(name,value){const bytes=Buffer.from(JSON.stringify(value,null,2));if(bytes.length>1024*1024)throw new Error('Receipt exceeds1MiB budget');fs.writeFileSync(path.join(out,name),bytes);}
const files=new Map([
 ['/qualia.js','docs/playground/qualia_core_db.js'],['/qualia_core_db_bg.wasm','docs/playground/qualia_core_db_bg.wasm'],
 ['/model.p64','models/SmolLM2-360M-Instruct-Q4_K_M.soa.p64'],
 ['/docs/playground/qualia_core_db.js','docs/playground/qualia_core_db.js'],['/docs/playground/qualia_core_db_bg.wasm','docs/playground/qualia_core_db_bg.wasm'],
 ...['qualia-cpu-worker.js','qualia-cpu-worker-client.js','etalon-worker-transport.js'].map(f=>['/docs/js/'+f,'docs/js/'+f])
]);
const result={startedUtc:new Date().toISOString(),kind:'Actual Chromium full WASM CPU runtime',checks:[],console:[],artifacts:Object.fromEntries(['docs/playground/qualia_core_db.js','docs/playground/qualia_core_db_bg.wasm',canonical,deployed,'models/SmolLM2-360M-Instruct-Q4_K_M.soa.p64'].map(file=>[file,hash(file)]))};
const server=http.createServer((req,res)=>{
 const url=new URL(req.url,'http://localhost');
 if(!process.env.NO_SAB){res.setHeader('Cross-Origin-Opener-Policy','same-origin');res.setHeader('Cross-Origin-Embedder-Policy','require-corp');}
 if(url.pathname==='/favicon.ico'){res.statusCode=204;return res.end();}
 if(url.pathname==='/'){res.setHeader('Content-Type','text/html');return res.end('<!doctype html><title>Qualia bounded browser validation</title>');}
 const rel=files.get(url.pathname); if(!rel){res.statusCode=404;return res.end();}
 const file=path.join(root,rel);res.setHeader('Content-Length',fs.statSync(file).size);res.setHeader('Content-Type',file.endsWith('.wasm')?'application/wasm':file.endsWith('.js')?'text/javascript':'application/octet-stream');
 fs.createReadStream(file).pipe(res);
});
(async()=>{await new Promise(r=>server.listen(0,'127.0.0.1',r)); const url='http://127.0.0.1:'+server.address().port;
 const browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH || 'C:/Users/Admin/AppData/Local/ms-playwright/chromium-1234/chrome-win64/chrome.exe',headless:true,args:['--disable-gpu']});
 try{const page=await browser.newPage();page.on('console',m=>{if(result.console.length<80)result.console.push(m.type()+': '+m.text());console.log('BROWSER',m.type(),m.text());});page.on('pageerror',e=>result.console.push('pageerror: '+e));await page.goto(url);
 const controls=await page.evaluate(async()=>{
  window.q=await import('/qualia.js');await q.default();const checks=[];
  async function rejects(name,fn,pattern){try{await fn();checks.push({name,passed:false,error:'unexpected success'});}catch(e){const error=String(e);checks.push({name,passed:pattern.test(error),error});}}
  checks.push({name:'default governed receipt',value:q.getBrowserExecutionReceipt()});
  await rejects('default governed CPU initialization rejects',()=>q.initializeCpuWasmEngine(new Uint8Array([0])),/governed inference is unavailable/);
  await rejects('developer requires explicit acknowledgement',()=>q.configureBrowserExecutionMode('developer',false),/acknowledgeUnguarded=true/);
  await rejects('invalid mode rejects',()=>q.configureBrowserExecutionMode('bad',true),/must be/);
  q.configureBrowserExecutionMode('developer',true);checks.push({name:'developer acknowledged receipt',value:q.getBrowserExecutionReceipt()});
  const token=q.beginBrowserWorkerRequest();checks.push({name:'fresh lifecycle token current',passed:q.isBrowserWorkerRequestCurrent(token)});
  q.configureBrowserExecutionMode('governed',false);checks.push({name:'mode change invalidates token',passed:!q.isBrowserWorkerRequestCurrent(token)&&!q.finishBrowserWorkerRequest(token)});
  q.configureBrowserExecutionMode('developer',true);const releaseToken=q.beginBrowserWorkerRequest();await q.releaseWebgpuEngine();checks.push({name:'release invalidates token',passed:!q.isBrowserWorkerRequestCurrent(releaseToken)&&!q.isWasmEngineReady()});
  return checks;
 });result.checks.push(...controls);console.log('CONTROLS',JSON.stringify(controls));
 const model=await page.evaluate(async()=>{
  const bytes=new Uint8Array(await (await fetch('/model.p64')).arrayBuffer());window.model=bytes;const start=performance.now();await q.initializeCpuWasmEngineWithContext(bytes,32);
  const receipt=q.getBrowserExecutionReceipt();return {name:'P64 CPU model initialization',passed:q.isWasmEngineReady()&&q.getWasmBackend()==='cpu-wasm'&&receipt.schema==='qualia.browser-execution.v2'&&receipt.outputProjection==='cpu'&&receipt.gpuTop1===false&&receipt.deviceToHostBytesPerToken===0&&receipt.readbackBasis==='no-device-transfers',backend:q.getWasmBackend(),vocab:q.getResidentTokenizerVocab(),elapsedMs:performance.now()-start,receipt};
 });result.checks.push(model);console.log('MODEL',JSON.stringify(model));
 const inference=await page.evaluate(async()=>{const pieces=[];const start=performance.now();const value=await q.inferWasmAsyncMeasured('Hello',2,p=>pieces.push(p));return {name:'P64 CPU real inference',passed:value.generatedTokens===2,value,pieces,elapsedMs:performance.now()-start};});result.checks.push(inference);console.log('INFERENCE',JSON.stringify(inference));
 const lifecycle=await page.evaluate(async()=>{
  const checks=[];const pending=q.inferWasmAsyncMeasured('Hello there',4,()=>{});q.configureBrowserExecutionMode('governed',false);
  try{await pending;checks.push({name:'pending inference mode-change cancellation',passed:false});}catch(e){checks.push({name:'pending inference mode-change cancellation',passed:/cancelled|governed/.test(String(e)),error:String(e)});}
  checks.push({name:'mode change releases model',passed:!q.isWasmEngineReady(),receipt:q.getBrowserExecutionReceipt()});
  try{await q.inferWasmAsyncMeasured('Hello',1,()=>{});checks.push({name:'governed inference rejects after mode change',passed:false});}catch(e){checks.push({name:'governed inference rejects after mode change',passed:/governed inference is unavailable/.test(String(e)),error:String(e)});}
  return checks;
 });result.checks.push(...lifecycle);console.log('LIFECYCLE',JSON.stringify(lifecycle));

 const worker=await Promise.race([page.evaluate(async()=>{
   const {QualiaCpuWorkerClient}=await import('/docs/js/qualia-cpu-worker-client.js');
   const guarded=new QualiaCpuWorkerClient();let defaultDenied=false;try{await guarded.initialize(new Uint8Array([1]));}catch(e){defaultDenied=/developerMode/.test(String(e));}
   const client=new QualiaCpuWorkerClient({developerMode:true,modelByteBudget:256*1024*1024});
   const bytes=model.buffer.slice(0);console.log('worker initialize starting');const details=await client.initialize(bytes);console.log('worker initialized',JSON.stringify(details));const transferred=bytes.byteLength===0;const pieces=[];
   const value=await client.infer('Hello',1,p=>pieces.push(p));await client.release();
   return {name:'dedicated worker real model/inference/release',passed:defaultDenied&&transferred&&value.generatedTokens===1&&!client.ready,defaultDenied,transferred,details,value,pieces,crossOriginIsolated};
 }),new Promise((_,reject)=>{const timer=setTimeout(()=>reject(new Error('Dedicated worker timeout after 45s')),45000);timer.unref();})]);result.checks.push(worker);console.log('WORKER',JSON.stringify(worker));
 }catch(e){result.error=String(e);console.error(e);}finally{await browser.close();server.close();result.completedUtc=new Date().toISOString();writeReceipt('browser-runtime-'+(process.env.NO_SAB?'fallback':'shared')+'.json',result);if(result.error||result.checks.some(c=>c.passed===false)||result.console.some(c=>c.startsWith('pageerror:')))process.exitCode=1;}
})();
