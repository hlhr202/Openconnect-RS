import {
  Accordion,
  AccordionItem,
  Button,
  Modal,
  ModalBody,
  ModalContent,
  ModalFooter,
  ModalHeader,
} from "@heroui/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
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
    <Modal
      size="lg"
      backdrop="blur"
      isOpen={!!prompt}
      isDismissable={false}
      isKeyboardDismissDisabled
      hideCloseButton
      scrollBehavior="inside"
    >
      <ModalContent>
        {prompt && (
          <>
            <ModalHeader
              className={isMismatch ? "text-danger" : "text-warning"}
            >
              {isMismatch
                ? "Server certificate has changed!"
                : "Unverified server certificate"}
            </ModalHeader>
            <ModalBody className="gap-3 text-small">
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
                  This server's certificate cannot be validated with a trusted
                  authority.{" "}
                  <b>
                    There is no guarantee that the server is the computer you
                    think it is.
                  </b>{" "}
                  If the information below is valid and you used another way
                  to validate it, choose "Accurate information" to remember it
                  and carry on connecting. Otherwise, cancel.
                </p>
              )}
              <div className="flex flex-col gap-1 rounded-lg bg-content2 p-3 font-mono text-tiny break-all">
                <span>
                  Host: {host}
                  {prompt.port ? `:${prompt.port}` : ""}
                </span>
                <span>Reason: {prompt.reason}</span>
                <span>Fingerprint: {prompt.fingerprint}</span>
                {isMismatch && (
                  <span className="text-danger">
                    Previously trusted: {prompt.storedFingerprint}
                  </span>
                )}
              </div>
              {prompt.details && (
                <Accordion isCompact variant="bordered">
                  <AccordionItem
                    key="details"
                    aria-label="Certificate details"
                    title="Certificate details"
                  >
                    <pre className="max-h-48 overflow-auto whitespace-pre-wrap text-tiny">
                      {prompt.details}
                    </pre>
                  </AccordionItem>
                </Accordion>
              )}
            </ModalBody>
            <ModalFooter>
              <Button
                variant="flat"
                isDisabled={isSubmitting}
                onPress={() => respond(false)}
              >
                Cancel
              </Button>
              <Button
                color={isMismatch ? "danger" : "warning"}
                isLoading={isSubmitting}
                onPress={() => respond(true)}
              >
                {isMismatch
                  ? "The key was changed by the administrator"
                  : "Accurate information"}
              </Button>
            </ModalFooter>
          </>
        )}
      </ModalContent>
    </Modal>
  );
};
