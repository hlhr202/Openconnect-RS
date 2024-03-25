import { invoke } from "@tauri-apps/api";
import { atom, useAtom } from "jotai";
import { useCallback, useMemo } from "react";

export interface OidcServer {
  name: string;
  authType: "oidc";
  server: string;
  issuer: string;
  clientId: string;
  clientSecret?: string;
  allowInsecure?: boolean,
  updatedAt?: string;
}

export interface PasswordServer {
  name: string;
  authType: "password";
  server: string;
  username: string;
  password: string;
  allowInsecure?: boolean,
  updatedAt?: string;
}

export interface StoredConfigs {
  default?: string | null;
  servers: (OidcServer | PasswordServer)[];
}

export const storedConfigsAtom = atom<StoredConfigs["servers"]>([]);
export const selectedNameAtom = atom<string | null>(null);
export const defaultNameAtom = atom<string | null>(null);

export const useStoredConfigs = () => {
  const [serverList, setServerList] = useAtom(storedConfigsAtom);
  const [selectedName, setSelectedName] = useAtom(selectedNameAtom);
  const [defaultName, setDefaultName] = useAtom(defaultNameAtom);

  const getStoredConfigs = useCallback(async () => {
    const configs = await invoke<StoredConfigs>("get_stored_configs");
    configs.servers.sort(
      (a, b) => b.updatedAt?.localeCompare(a.updatedAt ?? "") ?? 0
    );
    setServerList(configs.servers);
    setSelectedName(configs.default ?? configs.servers[0].name);
    setDefaultName(configs.default ?? null);
  }, [setServerList, setSelectedName]);

  const selectedServer = useMemo(() => {
    return serverList?.find((server) => server.name === selectedName);
  }, [selectedName, serverList]);

  return {
    defaultName,
    serverList,
    selectedName,
    getStoredConfigs,
    setSelectedName,
    selectedServer,
  };
};
