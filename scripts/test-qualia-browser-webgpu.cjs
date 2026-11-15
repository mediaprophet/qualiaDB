// Actual browser WebGPU qualification. Run only after obtaining the shared GPU slot.
// Set CHROMIUM_PATH and ARTIFACT_DIR as needed; never accepts a CPU fallback as success.
const http=require('node:http'),fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {chromium}=require('playwright');
const root=path.resolve(__dirname,'..');
const out=path.resolve(process.env.ARTIFACT_DIR||path.join(root,'target/qualia-browser-runtime'));
const allow=new Map([['/qualia.js','docs/playground/qualia_core_db.js'],['/qualia_core_db_bg.wasm','docs/playground/qualia_core_db_bg.wasm'],['/model.p64','models/SmolLM2-360M-Instruct-Q4_K_M.soa.p64']]);
allow.set('/topk.wgsl','coordination/worktrees/etalon-migration/crates/etalon-inference/src/projections/topk_reduction.wgsl');
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(path.join(root,p))).digest('hex');
const receipt={startedUtc:new Date().toISOString(),kind:'Actual Chromium WebGPU; developer mode',artifacts:Object.fromEntries([...allow.values()].map(p=>[p,hash(p)])),checks:[],console:[]};
receipt.qualificationNotes={executedProductionPath:'Per-chunk GPU GEMM projection with CPU streaming argmax; WebGPU backend, no CPU backend fallback',transferMeasurement:'Receipt v2 identifies GPU GEMM with CPU streaming argmax; complete device transfers remain unmeasured (null). Full-logits bytes are a tokenizer-vocabulary estimate. Separate shader regression validates GPU top-K only.'};
const server=http.createServer((req,res)=>{
 const pathname=new URL(req.url,'http://localhost').pathname;
 res.setHeader('Cross-Origin-Opener-Policy','same-origin');res.setHeader('Cross-Origin-Embedder-Policy','require-corp');
 if(pathname==='/'){res.setHeader('Content-Type','text/html');return res.end('<!doctype html><title>Qualia WebGPU qualification</title>');}
 if(pathname==='/favicon.ico'){res.statusCode=204;return res.end();}
 const rel=allow.get(pathname);if(!rel){res.statusCode=404;return res.end();}
 const file=path.join(root,rel);res.setHeader('Content-Type',file.endsWith('.wasm')?'application/wasm':file.endsWith('.js')?'text/javascript':'application/octet-stream');res.setHeader('Content-Length',fs.statSync(file).size);fs.createReadStream(file).pipe(res);
});
(async()=>{let browser;try{
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 // Enables Chromium's WebGPU API, without forcing a backend or software adapter.
 browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||'C:/Users/Admin/AppData/Local/ms-playwright/chromium-1234/chrome-win64/chrome.exe',headless:true,args:['--enable-unsafe-webgpu']});
 const page=await browser.newPage();page.on('console',m=>{if(receipt.console.length<100)receipt.console.push(m.type()+': '+m.text().slice(0,4000));console.log(m.type(),m.text().slice(0,1000));});page.on('pageerror',e=>receipt.console.push('pageerror: '+e));
 await page.goto('http://127.0.0.1:'+server.address().port);
 const probe=await page.evaluate(async()=>{
  if(!navigator.gpu)return {passed:false,error:'navigator.gpu unavailable'};
  const adapter=await navigator.gpu.requestAdapter({powerPreference:'high-performance',forceFallbackAdapter:false});
  if(!adapter)return {passed:false,error:'navigator.gpu.requestAdapter returned null'};
  const info=adapter.info||(adapter.requestAdapterInfo?await adapter.requestAdapterInfo():{});
  const identity={vendor:info.vendor,architecture:info.architecture,device:info.device,description:info.description,isFallbackAdapter:typeof info.isFallbackAdapter==='boolean'?info.isFallbackAdapter:(typeof adapter.isFallbackAdapter==='boolean'?adapter.isFallbackAdapter:null)};
  const device=await adapter.requestDevice();const uncaptured=[];device.addEventListener('uncapturederror',e=>uncaptured.push(String(e.error)));
  device.pushErrorScope('validation');
  const output=device.createBuffer({size:4,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});const readback=device.createBuffer({size:4,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
  const module=device.createShaderModule({code:'@group(0) @binding(0) var<storage,read_write> result: array<u32>; @compute @workgroup_size(1) fn main(){result[0]=7u;}'});
  const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module,entryPoint:'main'}});const group=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:output}}]});
  const encoder=device.createCommandEncoder();const pass=encoder.beginComputePass();pass.setPipeline(pipeline);pass.setBindGroup(0,group);pass.dispatchWorkgroups(1);pass.end();encoder.copyBufferToBuffer(output,0,readback,0,4);device.queue.submit([encoder.finish()]);await readback.mapAsync(GPUMapMode.READ);const value=new Uint32Array(readback.getMappedRange())[0];readback.unmap();const error=await device.popErrorScope();output.destroy();readback.destroy();device.destroy();
  return {passed:value===7&&!error&&!uncaptured.length,identity,features:[...adapter.features],value,error:error?String(error):null,uncaptured};
 });receipt.checks.push({name:'actual browser adapter and tiny GPU compute/readback',...probe});console.log('ADAPTER',JSON.stringify(probe));
 if(!probe.passed)throw new Error(probe.error||'GPU compute/readback failed');
 const topk=await page.evaluate(async()=>{
  const adapter=await navigator.gpu.requestAdapter({powerPreference:'high-performance',forceFallbackAdapter:false});
  const device=await adapter.requestDevice();const buffers=[];
  try{
   device.pushErrorScope('validation');
   const module=device.createShaderModule({code:await(await fetch('/topk.wgsl')).text()});
   const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module,entryPoint:'topk_block'}});
   const buffer=(size,usage)=>{const b=device.createBuffer({size,usage});buffers.push(b);return b;};
   const logits=buffer(16,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST),params=buffer(16,GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST);
   const values=buffer(8,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC),indices=buffer(8,GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC),readback=buffer(16,GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ);
   const group=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[logits,params,values,indices].map((b,binding)=>({binding,resource:{buffer:b}}))});
   const results=[];
   for(const input of [[-8,3,3,-2],[-8,-3,-3,-2]]){
    device.queue.writeBuffer(logits,0,new Float32Array(input));device.queue.writeBuffer(params,0,new Uint32Array([4,2,4,0]));
    const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();pass.setPipeline(pipeline);pass.setBindGroup(0,group);pass.dispatchWorkgroups(1);pass.end();
    encoder.copyBufferToBuffer(values,0,readback,0,8);encoder.copyBufferToBuffer(indices,0,readback,8,8);device.queue.submit([encoder.finish()]);
    await readback.mapAsync(GPUMapMode.READ);const bytes=readback.getMappedRange();results.push({values:[...new Float32Array(bytes,0,2)],indices:[...new Uint32Array(bytes,8,2)]});readback.unmap();
   }
   const error=await device.popErrorScope();return {passed:!error&&JSON.stringify(results)==='[{"values":[3,3],"indices":[1,2]},{"values":[-2,-3],"indices":[3,1]}]',results,error:error?String(error):null};
  }finally{buffers.forEach(b=>b.destroy());device.destroy();}
 });receipt.checks.push({name:'Chromium exact finite-minimum top-K shader: ties and negative logits',...topk});
 if(!topk.passed)throw new Error('Top-K Chromium regression failed: '+JSON.stringify(topk));
 const workload=page.evaluate(async()=>{
  const q=await import('/qualia.js');await q.default();q.configureBrowserExecutionMode('developer',true);
  const graphics=await q.probe_portal_graphics();
  const bytes=new Uint8Array(await(await fetch('/model.p64')).arrayBuffer());const start=performance.now();
  try{await q.initialize_webgpu_engine(bytes);const before=q.getBrowserExecutionReceipt();if(before.schema!=='qualia.browser-execution.v2'||before.gpuTop1!==false||before.outputProjection!=='gpu-gemm-cpu-streaming-argmax'||before.deviceToHostBytesPerToken!==null||before.readbackBasis!=='unmeasured')throw new Error('Incorrect active-path receipt '+JSON.stringify(before));if(q.getWasmBackend()!=='webgpu')throw new Error('Unexpected backend '+q.getWasmBackend());const pieces=[];const value=await q.inferWasmAsyncMeasured('Hello',1,p=>pieces.push(p));return {passed:q.isWebgpuEngineReady()&&value.generatedTokens===1,graphics,receipt:before,value,pieces,elapsedMs:performance.now()-start};}
  finally{await q.releaseWebgpuEngine();q.configureBrowserExecutionMode('governed',false);}
 });
 const result=await Promise.race([workload,new Promise((_,reject)=>{const timer=setTimeout(()=>reject(new Error('Browser WebGPU model/inference exceeded180s')),180000);timer.unref();})]);receipt.checks.push({name:'actual generated WASM WebGPU real P64 one-token inference',...result});console.log('MODEL',JSON.stringify(result));
}catch(e){receipt.error=String(e);process.exitCode=1;console.error(e);}finally{
 if(browser)await browser.close();server.close();receipt.completedUtc=new Date().toISOString();fs.mkdirSync(out,{recursive:true});const bytes=Buffer.from(JSON.stringify(receipt,null,2));if(bytes.length>1024*1024)throw new Error('Receipt exceeds1MiB budget');fs.writeFileSync(path.join(out,'browser-webgpu.json'),bytes);if(receipt.checks.some(c=>c.passed===false)||receipt.console.some(c=>c.startsWith('pageerror:')))process.exitCode=1;
}})();
