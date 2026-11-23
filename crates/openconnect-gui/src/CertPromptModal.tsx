import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { ShieldAlert, ShieldQuestion } from "lucide-react";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { useStoredConfigs } from "./state";
import { toastError } from "./lib/toast";

/** Payload of the `vpnCertPrompt` event (see `cert_prompt.rs`). */
interface CertPrompt {
  id: number;
  serverName: string;
  host?: string | null;
  port: number;
  fingerprint: string;
  /** Set when the server key changed since it was last trusted. */
  storedFingerprint?: string | null;
  reason: string;
  details?: string | null;
}

/**
 * Trust-on-first-use dialog shown when a server certificate cannot be validated by a
 * trusted authority, mirroring the original openconnect-gui behaviour.
 */
export const CertPromptModal = () => {
  const [queue, setQueue] = useState<CertPrompt[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { getStoredConfigs } = useStoredConfigs();
  const prompt = queue[0];

  useEffect(() => {
    const unlisten = listen<CertPrompt>("vpnCertPrompt", (event) => {
      setQueue((q) => [...q, event.payload]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const respond = useCallback(
    async (accept: boolean) => {
      if (!prompt) return;
      setIsSubmitting(true);
      try {
        await invoke("respond_cert_prompt", { id: prompt.id, accept });
        if (accept) {
          await getStoredConfigs();
        }
      } catch (e) {
        toastError(e);
      } finally {
        setIsSubmitting(false);
        setQueue((q) => q.slice(1));
      }
    },
    [prompt, getStoredConfigs]
  );

  const isMismatch = !!prompt?.storedFingerprint;
  const host = prompt?.host ?? prompt?.serverName ?? "";

  return (
    <Dialog open={!!prompt}>
      <DialogContent
        showCloseButton={false}
        onPointerDownOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
        className="flex max-h-[85vh] flex-col gap-0 overflow-hidden bg-background/95 p-0 backdrop-blur-xl sm:max-w-lg"
      >
        {prompt && (
          <>
            <DialogHeader className="border-b px-6 py-4">
              <DialogTitle
                className={`flex items-center gap-2 text-base ${
                  isMismatch ? "text-destructive" : "text-amber-400"
                }`}
              >
                {isMismatch ? (
                  <ShieldAlert className="size-5 shrink-0" />
                ) : (
                  <ShieldQuestion className="size-5 shrink-0" />
                )}
                <span>
                  {isMismatch
                    ? "Server certificate has changed!"
                    : "Unverified server certificate"}
                </span>
              </DialogTitle>
              <DialogDescription className="sr-only">
                Verify the server certificate fingerprint before continuing
              </DialogDescription>
            </DialogHeader>
            <div className="flex flex-1 flex-col gap-3 overflow-y-auto px-6 py-4 text-sm leading-relaxed">
              {isMismatch ? (
                <p>
                  The server <b>{prompt.serverName}</b> is known and associated
                  with a different key. It may be that the server has multiple
                  keys, the administrator replaced it, or{" "}
                  <b>you are (or were in the past) under attack</b>. Do you want
                  to proceed?
                </p>
              ) : (
                <p>
                  This server&apos;s certificate cannot be validated with a
                  trusted authority.{" "}
                  <b>
                    There is no guarantee that the server is the computer you
                    think it is.
                  </b>{" "}
                  If the information below is valid and you used another way to
                  validate it, choose &quot;Accurate information&quot; to
                  remember it and carry on connecting. Otherwise, cancel.
                </p>
              )}
              <div className="flex flex-col gap-1 rounded-lg border bg-muted/40 p-3 font-mono text-xs break-all">
                <span>
                  Host: {host}
                  {prompt.port ? `:${prompt.port}` : ""}
                </span>
                <span>Reason: {prompt.reason}</span>
                <span>Fingerprint: {prompt.fingerprint}</span>
                {isMismatch && (
                  <span className="text-destructive">
                    Previously trusted: {prompt.storedFingerprint}
                  </span>
                )}
              </div>
              {prompt.details && (
                <Accordion
                  type="single"
                  collapsible
                  className="rounded-lg border px-3"
                >
                  <AccordionItem value="details" className="border-b-0">
                    <AccordionTrigger className="py-2.5 text-sm hover:no-underline">
                      Certificate details
                    </AccordionTrigger>
                    <AccordionContent>
                      <pre className="max-h-48 overflow-auto whitespace-pre-wrap font-mono text-xs text-muted-foreground">
                        {prompt.details}
                      </pre>
                    </AccordionContent>
                  </AccordionItem>
                </Accordion>
              )}
            </div>
            <DialogFooter className="mx-0 mb-0 border-t px-6 py-3">
              <Button
                type="button"
                variant="outline"
                disabled={isSubmitting}
                onClick={() => respond(false)}
              >
                Cancel
              </Button>
              <Button
                type="button"
                variant={isMismatch ? "destructive" : "default"}
                className={
                  isMismatch
                    ? undefined
                    : "bg-amber-500 text-zinc-950 hover:bg-amber-400"
                }
                disabled={isSubmitting}
                onClick={() => respond(true)}
              >
                {isSubmitting && <Spinner className="size-3.5" />}
                {isMismatch
                  ? "The key was changed by the administrator"
                  : "Accurate information"}
              </Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
};
