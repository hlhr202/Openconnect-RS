import {
  Button,
  Divider,
  // Modal,
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

export interface OidcServer {
  name: string;
  authType: "oidc";
  server: string;
  issuer: string;
  clientId: string;
  clientSecret?: string | null;
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

interface IProps {
  onConnect: (server: OidcServer | PasswordServer) => void;
}

export const ServerSelector = (props: IProps) => {
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

  const handleConnect = useCallback(() => {
    if (selectedServer) {
      props.onConnect(selectedServer);
    }
  }, [props, selectedServer]);

  useEffect(() => {
    getStoredConfigs();
  }, [getStoredConfigs]);

  return (
    <section className="h-full w-full flex flex-col">
      <div className="w-full flex">
        {!!serverList?.length && (
          <Select
            size="md"
            color="success"
            className="flex-1"
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
        <Button size="md" color="primary" className="ml-2">Manage Server</Button>
      </div>
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
      <Button
        color="primary"
        className="w-full mt-auto"
        onClick={handleConnect}
      >
        Connect
      </Button>
    </section>
  );
};

// export const ServerEditorModal = () => {
//   return (
//     <Modal>
//       {(onclose) => {

//       }}
//     </Modal>
//   )
// }