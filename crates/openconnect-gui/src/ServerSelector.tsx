import {
  Button,
  Divider,
  Select,
  SelectItem,
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
} from "@nextui-org/react";
import { invoke } from "@tauri-apps/api";
import { useCallback, useEffect, useMemo, useState } from "react";

interface OidcServer {
  name: string;
  authType: "oidc";
  server: string;
  issuer: string;
  clientId: string;
  clientSecret?: string | null;
}

interface PasswordServer {
  name: string;
  authType: "password";
  server: string;
  username: string;
  password: string;
}

interface StoredConfigs {
  default?: string | null;
  servers: (OidcServer | PasswordServer)[];
}

export const ServerSelector = () => {
  const [selectedName, setSelectedName] = useState<string | null>(null);

  const [serverList, setServerList] = useState<StoredConfigs["servers"]>();

  const getStoredConfigs = useCallback(async () => {
    const configs = await invoke<StoredConfigs>("get_stored_configs");
    setServerList(configs.servers);
    setSelectedName(configs.default ?? configs.servers[0].name);
  }, []);

  const selectedServer = useMemo(() => {
    return serverList?.find((server) => server.name === selectedName);
  }, [selectedName, serverList]);

  useEffect(() => {
    getStoredConfigs();
  }, [getStoredConfigs]);

  return (
    <section className="h-full flex flex-col">
      {!!serverList?.length && (
        <Select
          size="lg"
          color="success"
          selectionMode="single"
          selectedKeys={selectedName ? [selectedName] : []}
        >
          {serverList?.map((server) => (
            <SelectItem color="default" key={server.name} value={server.name}>
              {server.name}
            </SelectItem>
          ))}
        </Select>
      )}
      <Divider className="mt-3 mb-3"></Divider>
      <Table removeWrapper>
        <TableHeader>
          <TableColumn>Type</TableColumn>
          <TableColumn>Server</TableColumn>
        </TableHeader>
        <TableBody>
          <TableRow>
            <TableCell>{selectedServer?.authType}</TableCell>
            <TableCell>{selectedServer?.server}</TableCell>
          </TableRow>
        </TableBody>
      </Table>
      <Button color="primary" className="w-full mt-auto">
        Connect
      </Button>
    </section>
  );
};
