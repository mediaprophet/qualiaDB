import '../frb_generated.dart';

Future<String> buildAnatomyGraphContext({
  required String qappName,
  required String userPrompt,
  required String agentReply,
  String? dicomFilePath,
}) {
  if (dicomFilePath != null && dicomFilePath.trim().isNotEmpty) {
    return RustApi.instance.api
        .crateApiQualiaApiBuildAnatomyGraphContextJsonWithDicom(
      qappName: qappName,
      userPrompt: userPrompt,
      agentReply: agentReply,
      dicomFilePath: dicomFilePath,
    );
  }
  return RustApi.instance.api.crateApiQualiaApiBuildAnatomyGraphContextJson(
    qappName: qappName,
    userPrompt: userPrompt,
    agentReply: agentReply,
  );
}

Future<String> parseDicomMetadata({required String filePath}) {
  return RustApi.instance.api.crateApiQualiaApiParseDicomMetadataJson(
    filePath: filePath,
  );
}

Future<String> buildDicomOverlaySpec({required String filePath}) {
  return RustApi.instance.api.crateApiQualiaApiBuildDicomOverlaySpecJson(
    filePath: filePath,
  );
}

Future<String> launchInstalledQappWithContext({
  required String qappName,
  String? entrypoint,
  String? surface,
  String? payloadJson,
  String? source,
}) {
  return RustApi.instance.api.crateApiQualiaApiLaunchInstalledQappWithContext(
    qappName: qappName,
    entrypoint: entrypoint,
    surface: surface,
    payloadJson: payloadJson,
    source: source,
  );
}

Future<String> inspectInstalledQappReadiness({
  required String qappName,
}) {
  return RustApi.instance.api.crateApiQualiaApiInspectInstalledQappReadiness(
    qappName: qappName,
  );
}

Future<List<String>> listInstalledOntologyArtifacts() {
  return RustApi.instance.api.crateApiQualiaApiListInstalledOntologyArtifacts();
}

Future<String> removeInstalledOntology({
  required String ontologyId,
}) {
  return RustApi.instance.api.crateApiQualiaApiRemoveInstalledOntology(
    ontologyId: ontologyId,
  );
}

Future<void> unloadActiveModel() {
  return RustApi.instance.api.crateApiQualiaApiUnloadActiveModel();
}

Future<String> removeInstalledModel({
  required String modelId,
}) {
  return RustApi.instance.api.crateApiQualiaApiRemoveInstalledModel(
    modelId: modelId,
  );
}

Future<String> testSparqlEndpoint({
  required String endpointOrId,
}) {
  return RustApi.instance.api.crateApiQualiaApiTestSparqlEndpoint(
    endpointOrId: endpointOrId,
  );
}
