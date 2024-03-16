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
  useDisclosure,
} from "@nextui-org/react";
import { useCallback, useEffect } from "react";
import { ServerEditorModal } from "./ServerEditorModal";
import { OidcServer, PasswordServer, useStoredConfigs } from "./state";

interface IProps {
  onConnect: (server: OidcServer | PasswordServer) => void;
}

export const ServerSelector = (props: IProps) => {
  const {
    getStoredConfigs,
    selectedServer,
    selectedName,
    serverList,
    setSelectedName,
  } = useStoredConfigs();

  const handleConnect = useCallback(() => {
    if (selectedServer) {
      props.onConnect(selectedServer);
    }
  }, [props, selectedServer]);

  const { isOpen, onOpen, onOpenChange } = useDisclosure();

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
            onSelectionChange={(keys) => {
              const name = Array.from(keys as Set<string>)[0];
              if (name) {
                setSelectedName(name);
              }
            }}
          >
            {serverList?.map((server) => (
              <SelectItem color="default" key={server.name} value={server.name}>
                {server.name}
              </SelectItem>
            ))}
          </Select>
        )}
        <Button size="md" color="primary" className="ml-2" onClick={onOpen}>
          Manage Server
        </Button>
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

      <ServerEditorModal
        isOpen={isOpen}
        onOpen={onOpen}
        onOpenChange={onOpenChange}
      />
    </section>
  );
};
