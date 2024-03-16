import {
  Button,
  Divider,
  Select,
  SelectItem,
  useDisclosure,
} from "@nextui-org/react";
import { FC, PropsWithChildren, useEffect } from "react";
import { ServerEditorModal } from "./ServerEditorModal";
import { useStoredConfigs } from "./state";

export const ServerSelector = () => {
  const {
    getStoredConfigs,
    selectedServer,
    selectedName,
    serverList,
    setSelectedName,
  } = useStoredConfigs();

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
            // color="success"
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
      <div className="flex flex-col gap-6 p-1">
        <InfoRow label="Server Type">{selectedServer?.authType}</InfoRow>
        <InfoRow label="Server URL">{selectedServer?.server}</InfoRow>
        {selectedServer?.authType === "password" && (
          <>
            <InfoRow label="Username">{selectedServer?.username}</InfoRow>
            <InfoRow label="Password">{"********"}</InfoRow>
          </>
        )}
        {selectedServer?.authType === "oidc" && (
          <>
            <InfoRow label="Issuer">{selectedServer?.issuer}</InfoRow>
            <InfoRow label="Client ID">{selectedServer?.clientId}</InfoRow>
          </>
        )}
      </div>

      <ServerEditorModal
        isOpen={isOpen}
        onOpen={onOpen}
        onOpenChange={onOpenChange}
      />
    </section>
  );
};

const InfoRow: FC<PropsWithChildren<{ label: string }>> = (props) => {
  return (
    <div className="flex gap-4">
      <div className="w-[100px]">{props.label}:</div>
      <Divider orientation="vertical" />
      <div className="max-w-[400px] overflow-hidden text-ellipsis whitespace-nowrap">
        {props.children}
      </div>
    </div>
  );
};
