import { FC, PropsWithChildren, useEffect, useState } from "react";
import { useKey } from "react-use";
import { Settings2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { ServerEditorModal } from "./ServerEditorModal";
import { useStoredConfigs } from "./state";

const isInteractiveTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false;
  return !!target.closest(
    "input, textarea, select, [contenteditable='true'], [role='dialog'], [role='listbox']"
  );
};

export const ServerSelector = () => {
  const {
    getStoredConfigs,
    selectedServer,
    selectedName,
    serverList,
    setSelectedName,
  } = useStoredConfigs();

  const [isOpen, setIsOpen] = useState(false);

  useKey(
    "Tab",
    (evt) => {
      if (isOpen || isInteractiveTarget(evt.target)) return;
      evt.preventDefault();
      if (serverList.length) {
        const currentIndex = serverList.findIndex(
          ({ name }) => name === selectedName
        );
        if (currentIndex !== -1) {
          const nextIndex =
            currentIndex === serverList.length - 1 ? 0 : currentIndex + 1;
          const next = serverList[nextIndex].name;
          setSelectedName(next);
        }
      }
    },
    undefined,
    [isOpen, serverList, selectedName, setSelectedName]
  );

  useKey(
    "m",
    (evt) => {
      if (isOpen || isInteractiveTarget(evt.target)) return;
      setIsOpen(true);
    },
    undefined,
    [isOpen]
  );

  useEffect(() => {
    getStoredConfigs();
  }, [getStoredConfigs]);

  return (
    <section className="flex h-full w-full flex-col">
      <div className="flex w-full items-center gap-2">
        {!!serverList?.length && (
          <Select
            value={selectedName ?? ""}
            onValueChange={(name) => {
              if (name) {
                setSelectedName(name);
              }
            }}
          >
            <SelectTrigger
              aria-label="VPN Server"
              className="h-9 flex-1 bg-background/50"
            >
              <SelectValue placeholder="Select a VPN server" />
            </SelectTrigger>
            <SelectContent position="popper">
              {serverList.map((server) => (
                <SelectItem key={server.name} value={server.name}>
                  {server.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
        <Button
          type="button"
          className="h-9 shrink-0"
          onClick={() => setIsOpen(true)}
        >
          <Settings2 className="size-4" />
          Manage Server [M]
        </Button>
      </div>

      <Separator className="my-4 bg-white/10" />

      <div className="flex flex-col gap-5 px-1">
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
        onOpen={() => setIsOpen(true)}
        onOpenChange={setIsOpen}
      />
    </section>
  );
};

const InfoRow: FC<PropsWithChildren<{ label: string }>> = (props) => {
  return (
    <div className="flex items-center gap-4 text-sm">
      <div className="w-[100px] shrink-0 text-muted-foreground">
        {props.label}:
      </div>
      <Separator orientation="vertical" className="h-4 bg-white/15" />
      <div className="min-w-0 flex-1 truncate text-foreground">
        {props.children}
      </div>
    </div>
  );
};
