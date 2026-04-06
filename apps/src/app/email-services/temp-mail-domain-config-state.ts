export type TempMailDomainConfigFormValue = {
  id: string;
  name: string;
  zoneId: string;
  domainBase: string;
  subdomainMode: string;
  subdomainLength: string;
  subdomainPrefix: string;
  syncCloudflareEnabled: boolean;
  requireCloudflareSync: boolean;
};

type DomainConfigStateResult = {
  domainConfigs: TempMailDomainConfigFormValue[];
  selectedId: string | null;
};

const DEFAULT_SUBDOMAIN_MODE = "random";
const DEFAULT_SUBDOMAIN_LENGTH = "6";
const DEFAULT_SUBDOMAIN_PREFIX = "tm";

function createEmptyDomainConfig(
  createId: () => string
): TempMailDomainConfigFormValue {
  return {
    id: createId(),
    name: "",
    zoneId: "",
    domainBase: "",
    subdomainMode: DEFAULT_SUBDOMAIN_MODE,
    subdomainLength: DEFAULT_SUBDOMAIN_LENGTH,
    subdomainPrefix: DEFAULT_SUBDOMAIN_PREFIX,
    syncCloudflareEnabled: true,
    requireCloudflareSync: true,
  };
}

export function selectInitialDomainConfigId(
  domainConfigs: TempMailDomainConfigFormValue[],
  currentSelectedId: string | null
): string | null {
  if (currentSelectedId && domainConfigs.some((item) => item.id === currentSelectedId)) {
    return currentSelectedId;
  }
  return domainConfigs[0]?.id ?? null;
}

export function addDomainConfig(
  domainConfigs: TempMailDomainConfigFormValue[],
  createId: () => string
): DomainConfigStateResult {
  const nextItem = createEmptyDomainConfig(createId);
  return {
    domainConfigs: [...domainConfigs, nextItem],
    selectedId: nextItem.id,
  };
}

export function duplicateDomainConfig(
  domainConfigs: TempMailDomainConfigFormValue[],
  sourceId: string,
  createId: () => string,
  currentSelectedId: string | null = null
): DomainConfigStateResult {
  const index = domainConfigs.findIndex((item) => item.id === sourceId);
  if (index < 0) {
    return {
      domainConfigs,
      selectedId: selectInitialDomainConfigId(domainConfigs, currentSelectedId),
    };
  }

  const source = domainConfigs[index];
  const copyName = source.name ? `${source.name}-副本` : "";
  const copy: TempMailDomainConfigFormValue = {
    ...source,
    id: createId(),
    name: copyName,
  };
  const next = [...domainConfigs];
  next.splice(index + 1, 0, copy);
  return {
    domainConfigs: next,
    selectedId: copy.id,
  };
}

export function removeDomainConfig(
  domainConfigs: TempMailDomainConfigFormValue[],
  removeId: string,
  currentSelectedId: string | null
): DomainConfigStateResult {
  const index = domainConfigs.findIndex((item) => item.id === removeId);
  if (index < 0) {
    return {
      domainConfigs,
      selectedId: selectInitialDomainConfigId(domainConfigs, currentSelectedId),
    };
  }

  const next = domainConfigs.filter((item) => item.id !== removeId);
  const fallback = next[index]?.id ?? next[index - 1]?.id ?? null;
  return {
    domainConfigs: next,
    selectedId:
      currentSelectedId === removeId
        ? fallback
        : selectInitialDomainConfigId(next, currentSelectedId),
  };
}
