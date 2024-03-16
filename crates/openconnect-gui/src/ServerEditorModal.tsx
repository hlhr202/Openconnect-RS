import { Button } from "@nextui-org/button";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  Listbox,
  ListboxItem,
  Card,
  CardBody,
  ModalFooter,
} from "@nextui-org/react";
import { useAtom } from "jotai";
import { useState } from "react";
import { ServerEditor } from "./ServerEditor";
import { storedConfigsAtom } from "./state";

interface IModalProps {
  isOpen: boolean;
  onOpen: () => void;
  onOpenChange: (open: boolean) => void;
}

export interface FormParams {
  mode: "add" | "edit";
  name?: string;
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
  return (
    <Modal
      size="sm"
      isOpen={props.isOpen}
      onOpenChange={props.onOpenChange}
      onClose={handleClose}
      className="min-w-[800px] dark bg-background text-foreground bg-opacity-90"
    >
      <ModalContent>
        {(closeModal) => {
          return (
            <>
              <ModalHeader>Manager Servers</ModalHeader>
              <ModalBody>
                <Button
                  size="sm"
                  color="primary"
                  onClick={() => {
                    setSelectedName(null);
                    setFormParams({ mode: "add" });
                  }}
                >
                  Add New Server Config
                </Button>
                <div className="flex gap-1">
                  <Card>
                    <CardBody className="min-w-[200px]">
                      <Listbox
                        variant="flat"
                        color="primary"
                        topContent="Server List"
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
                          <ListboxItem key={server.name} value={server.name}>
                            {server.name}
                          </ListboxItem>
                        ))}
                      </Listbox>
                    </CardBody>
                  </Card>
                  {formParams ? (
                    <Card className="flex-1 ml-5">
                      <CardBody>
                        <ServerEditor {...formParams} />
                      </CardBody>
                    </Card>
                  ) : null}
                </div>
              </ModalBody>
              <ModalFooter>
                <Button onClick={closeModal} size="sm">
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
