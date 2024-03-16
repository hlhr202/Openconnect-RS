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
}

export interface PasswordServer {
  name: string;
  authType: "password";
  server: string;
  username: string;
  password: string;
}

export interface StoredConfigs {
  default?: string | null;
  servers: (OidcServer | PasswordServer)[];
}

export const storedConfigsAtom = atom<StoredConfigs["servers"]>([]);
export const defaultServerAtom = atom<string | null>(null);

export const useStoredConfigs = () => {
  const [serverList, setServerList] = useAtom(storedConfigsAtom);
  const [selectedName, setSelectedName] = useAtom(defaultServerAtom);

  const getStoredConfigs = useCallback(async () => {
    const configs = await invoke<StoredConfigs>("get_stored_configs");
    setServerList(configs.servers);
    setSelectedName(configs.default ?? configs.servers[0].name);
  }, [setServerList, setSelectedName]);

  const selectedServer = useMemo(() => {
    return serverList?.find((server) => server.name === selectedName);
  }, [selectedName, serverList]);

  return {
    serverList,
    selectedName,
    getStoredConfigs,
    setSelectedName,
    selectedServer,
  };
};
