import {
  Button,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  Listbox,
  ListboxItem,
  Card,
  CardBody,
  ModalFooter,
  Divider,
} from "@heroui/react";
import { useAtom } from "jotai";
import { useState } from "react";
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
  const handleClose = () => {
    setSelectedName(null);
    setFormParams(null);
    props.onOpenChange(false);
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
    <Modal
      size="sm"
      backdrop="blur"
      shadow="lg"
      hideCloseButton
      isOpen={props.isOpen}
      onOpenChange={props.onOpenChange}
      onClose={handleClose}
      className="min-w-[800px] min-h-[620px] dark bg-background text-foreground bg-opacity-90"
    >
      <ModalContent>
        {(closeModal) => {
          return (
            <>
              <ModalHeader className="select-none">Manager Servers</ModalHeader>
              <ModalBody>
                <div className="flex gap-4">
                  <Button
                    size="sm"
                    color="secondary"
                    className="flex-1"
                    onPress={() => {
                      navigator.clipboard
                        .readText()
                        .then(handlePaste)
                        .catch(() => {});
                    }}
                  >
                    Import config from clipboard
                  </Button>
                  <Button
                    size="sm"
                    color="primary"
                    className="flex-1"
                    onPress={() => {
                      setSelectedName(null);
                      setFormParams({ mode: "add" });
                    }}
                  >
                    Add New Server Config
                  </Button>
                </div>
                <div className="flex gap-1 flex-1">
                  <Card>
                    <CardBody className="min-w-[200px] overflow-auto">
                      <Listbox
                        aria-label="Server List"
                        variant="shadow"
                        color="primary"
                        disallowEmptySelection
                        topContent={
                          <>
                            <span className="select-none p-1">Server List</span>
                            <Divider />
                          </>
                        }
                        selectedKeys={selectedName ? [selectedName] : []}
                        selectionMode="single"
                        onSelectionChange={(keys) => {
                          const name = Array.from(keys as Set<string>)[0];
                          if (name) {
                            setSelectedName(name);
                            setFormParams({ mode: "edit", name });
                          } else {
                            setSelectedName(null);
                            setFormParams(null);
                          }
                        }}
                      >
                        {serverList.map((server) => (
                          <ListboxItem key={server.name}>
                            {server.name}
                          </ListboxItem>
                        ))}
                      </Listbox>
                    </CardBody>
                  </Card>
                  <Card className="flex-1 ml-5 min-h-full">
                    <CardBody>
                      {formParams && <ServerEditor {...formParams} />}
                    </CardBody>
                  </Card>
                </div>
              </ModalBody>
              <ModalFooter>
                <Button onPress={closeModal} size="sm" className="w-full">
                  Close
                </Button>
              </ModalFooter>
            </>
          );
        }}
      </ModalContent>
    </Modal>
  );
};
