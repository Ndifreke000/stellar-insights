export interface NPMPublishingSetupParams {
  packageName: string;
  version: string;
  registryUrl?: string;
  publishToken?: string;
}

export interface NPMPublishingSetupResult {
  success: true;
  data: {
    packageName: string;
    version: string;
    registryUrl: string;
    configuredAt: string;
  };
}
