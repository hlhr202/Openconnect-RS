import { Button } from "@nextui-org/button";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Link,
} from "@nextui-org/react";

interface IModalProps {
  isOpen: boolean;
  onOpen: () => void;
  onOpenChange: (open: boolean) => void;
}

export const AboutModal = (props: IModalProps) => {
  const handleClose = () => {
    props.onOpenChange(false);
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
              <ModalHeader className="select-none">About</ModalHeader>
              <ModalBody>
                <div className="flex gap-4 flex-col break-words">
                  <p>
                    This Application is created using Rust, and depends on
                    various opensource libraries and tools, some of them are
                    listed below:
                  </p>
                  <ul className="list-disc list-inside">
                    <li>
                      <Link
                        className="text-blue-400"
                        href="https://gitlab.com/openconnect/openconnect"
                        target="_blank"
                      >
                        OpenConnect (with all its dependencies)
                      </Link>
                    </li>
                    <li>
                      <Link
                        className="text-blue-400"
                        href="https://gcc.gnu.org/"
                        target="_blank"
                      >
                        GCC as compiler tool
                      </Link>
                    </li>
                    <li>
                      <Link
                        className="text-blue-400"
                        href="https://www.msys2.org/"
                        target="_blank"
                      >
                        MSYS2 as build environment under Windows
                      </Link>
                    </li>
                    <li>
                      <Link
                        className="text-blue-400"
                        href="https://github.com/microsoft/windows-rs"
                        target="_blank"
                      >
                        Windows-rs
                      </Link>
                    </li>
                    <li>
                      <Link
                        className="text-blue-400"
                        href="https://tauri.app/"
                        target="_blank"
                      >
                        Tauri as GUI library
                      </Link>
                    </li>
                    <li>
                      <Link
                        className="text-blue-400"
                        href="https://nextui.org/"
                        target="_blank"
                      >
                        NextUI
                      </Link>
                    </li>
                  </ul>
                  <p>
                    Since some of the core dependencies are released under{" "}
                    <Link
                      className="text-blue-400"
                      href="https://www.gnu.org/licenses/gpl-3.0.html#license-text"
                      target="_blank"
                    >
                      GPL
                    </Link>{" "}
                    /{" "}
                    <Link
                      className="text-blue-400"
                      href="https://www.gnu.org/licenses/lgpl-3.0.html"
                      target="_blank"
                    >
                      LGPL
                    </Link>{" "}
                    license, the source code of this application is also
                    released under LGPL.
                  </p>
                  <p>
                    For compilation issues or bug report, please contact{" "}
                    <Link
                      className="text-blue-400"
                      target="_blank"
                      href="https://github.com/hlhr202"
                    >
                      Github @ hlhr202
                    </Link>{" "}
                    or{" "}
                    <Link
                      className="text-blue-400"
                      target="_blank"
                      href="mailto:hlhr202@163.com"
                    >
                      My Email
                    </Link>
                  </p>
                </div>
              </ModalBody>
              <ModalFooter>
                <Button
                  onClick={closeModal}
                  size="sm"
                  className="w-full"
                  color="primary"
                >
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
