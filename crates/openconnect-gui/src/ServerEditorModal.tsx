import { useAtom } from "jotai";
import { useState } from "react";
import { Check, ClipboardPaste, Plus, Server } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";
import { FormParams, ServerEditor } from "./ServerEditor";
import { storedConfigsAtom } from "./state";
import { decodeUtf8Base64 } from "./lib/base64";
import { toastError } from "./lib/toast";

interface IModalProps {
  isOpen: boolean;
  onOpen: () => void;
  onOpenChange: (open: boolean) => void;
}

export const ServerEditorModal = (props: IModalProps) => {
  const [serverList] = useAtom(storedConfigsAtom);
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const [formParams, setFormParams] = useState<FormParams | null>(null);

  const handleOpenChange = (open: boolean) => {
    if (!open) {
      setSelectedName(null);
      setFormParams(null);
    }
    props.onOpenChange(open);
  };

  const handlePaste = (base64: string) => {
    setSelectedName(null);
    setFormParams(null);
    try {
      const decoded = decodeUtf8Base64(base64);
      const server = JSON.parse(decoded);
      setFormParams({ mode: "add", addFromImport: server });
    } catch {
      toastError({
        code: "PASTE_ERROR",
        message: "Cannot decode your input to a valid server",
      });
    }
  };

  return (
    <Dialog open={props.isOpen} onOpenChange={handleOpenChange}>
      <DialogContent
        showCloseButton={false}
        className="flex h-[620px] flex-col gap-0 overflow-hidden bg-background/95 p-0 backdrop-blur-xl sm:max-w-[800px]"
      >
        <DialogHeader className="border-b px-6 py-4">
          <DialogTitle className="select-none text-lg">
            Manage Servers
          </DialogTitle>
          <DialogDescription className="sr-only">
            Add, import, edit, or delete VPN server configurations
          </DialogDescription>
        </DialogHeader>

        <div className="flex min-h-0 flex-1 flex-col gap-4 px-6 py-4">
          <div className="flex shrink-0 gap-3">
            <Button
              type="button"
              size="sm"
              variant="secondary"
              className="flex-1"
              onClick={() => {
                navigator.clipboard
                  .readText()
                  .then(handlePaste)
                  .catch(() => {});
              }}
            >
              <ClipboardPaste className="size-4" />
              Import config from clipboard
            </Button>
            <Button
              type="button"
              size="sm"
              className="flex-1"
              onClick={() => {
                setSelectedName(null);
                setFormParams({ mode: "add" });
              }}
            >
              <Plus className="size-4" />
              Add New Server Config
            </Button>
          </div>

          <div className="flex min-h-0 flex-1 gap-4">
            <div className="flex w-[220px] shrink-0 flex-col overflow-hidden rounded-xl border bg-card">
              <div className="px-3.5 py-2.5 text-xs font-medium text-muted-foreground select-none">
                Server List
              </div>
              <Separator />
              <ScrollArea className="min-h-0 flex-1">
                <div
                  role="listbox"
                  aria-label="Server List"
                  className="flex flex-col gap-1 p-2"
                >
                  {serverList.length === 0 ? (
                    <div className="px-2 py-6 text-center text-xs text-muted-foreground">
                      No servers configured
                    </div>
                  ) : (
                    serverList.map((server) => {
                      const isSelected = selectedName === server.name;
                      return (
                        <button
                          key={server.name}
                          type="button"
                          role="option"
                          aria-selected={isSelected}
                          onClick={() => {
                            setSelectedName(server.name);
                            setFormParams({ mode: "edit", name: server.name });
                          }}
                          className={cn(
                            "flex w-full items-center justify-between gap-2 rounded-lg px-2.5 py-2 text-left text-sm transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
                            isSelected
                              ? "bg-primary text-primary-foreground font-medium"
                              : "text-foreground/90 hover:bg-muted"
                          )}
                        >
                          <span className="truncate">{server.name}</span>
                          {isSelected && <Check className="size-4 shrink-0" />}
                        </button>
                      );
                    })
                  )}
                </div>
              </ScrollArea>
            </div>

            <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-xl border bg-card p-4">
              {formParams ? (
                <ServerEditor {...formParams} />
              ) : (
                <div className="flex h-full flex-col items-center justify-center gap-2 text-center text-sm text-muted-foreground">
                  <Server className="size-8 opacity-40" />
                  <p>Select a server from the list or add a new configuration</p>
                </div>
              )}
            </div>
          </div>
        </div>

        <DialogFooter className="mx-0 mb-0 border-t px-6 py-3">
          <Button
            type="button"
            variant="outline"
            onClick={() => handleOpenChange(false)}
            className="w-full"
          >
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
