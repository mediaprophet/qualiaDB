export function compatibilityOnlyWorkerSource(source) {
  return `
const diagnosticGpu = globalThis.navigator?.gpu;
if (diagnosticGpu) {
  const request = diagnosticGpu.requestAdapter.bind(diagnosticGpu);
  diagnosticGpu.requestAdapter = options => options?.featureLevel === 'compatibility'
    ? request(options) : Promise.resolve(null);
}
${source}`;
}
