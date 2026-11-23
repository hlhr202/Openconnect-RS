import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface IModalProps {
  isOpen: boolean;
  onOpen: () => void;
  onOpenChange: (open: boolean) => void;
}

const linkClassName =
  "text-sky-400 underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50 rounded-xs";

export const AboutModal = (props: IModalProps) => {
  return (
    <Dialog open={props.isOpen} onOpenChange={props.onOpenChange}>
      <DialogContent
        showCloseButton={false}
        className="flex h-[580px] flex-col gap-0 overflow-hidden bg-background/95 p-0 backdrop-blur-xl sm:max-w-[760px]"
      >
        <DialogHeader className="border-b px-6 py-4">
          <DialogTitle className="select-none text-lg">About</DialogTitle>
          <DialogDescription className="sr-only">
            Open-source dependencies, license, and contact information
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-1 flex-col gap-4 overflow-y-auto px-6 py-5 text-sm leading-relaxed text-foreground/90 break-words">
          <p>
            This Application is created using Rust, and depends on various
            opensource libraries and tools, some of them are listed below:
          </p>
          <ul className="list-inside list-disc space-y-1.5">
            <li>
              <a
                className={linkClassName}
                href="https://gitlab.com/openconnect/openconnect"
                target="_blank"
                rel="noreferrer"
              >
                OpenConnect (with all its dependencies)
              </a>
            </li>
            <li>
              <a
                className={linkClassName}
                href="https://gcc.gnu.org/"
                target="_blank"
                rel="noreferrer"
              >
                GCC as compiler tool
              </a>
            </li>
            <li>
              <a
                className={linkClassName}
                href="https://www.msys2.org/"
                target="_blank"
                rel="noreferrer"
              >
                MSYS2 as build environment under Windows
              </a>
            </li>
            <li>
              <a
                className={linkClassName}
                href="https://github.com/microsoft/windows-rs"
                target="_blank"
                rel="noreferrer"
              >
                Windows-rs
              </a>
            </li>
            <li>
              <a
                className={linkClassName}
                href="https://tauri.app/"
                target="_blank"
                rel="noreferrer"
              >
                Tauri as GUI library
              </a>
            </li>
            <li>
              <a
                className={linkClassName}
                href="https://ui.shadcn.com/"
                target="_blank"
                rel="noreferrer"
              >
                shadcn/ui &amp; Tailwind CSS
              </a>
            </li>
          </ul>
          <p>
            Since some of the core dependencies are released under{" "}
            <a
              className={linkClassName}
              href="https://www.gnu.org/licenses/gpl-3.0.html#license-text"
              target="_blank"
              rel="noreferrer"
            >
              GPL
            </a>{" "}
            /{" "}
            <a
              className={linkClassName}
              href="https://www.gnu.org/licenses/lgpl-3.0.html"
              target="_blank"
              rel="noreferrer"
            >
              LGPL
            </a>{" "}
            license, the source code of this application is also released under
            LGPL.
          </p>
          <p>
            For compilation issues or bug report, please contact{" "}
            <a
              className={linkClassName}
              target="_blank"
              rel="noreferrer"
              href="https://github.com/hlhr202"
            >
              Github @ hlhr202
            </a>{" "}
            or{" "}
            <a
              className={linkClassName}
              target="_blank"
              rel="noreferrer"
              href="mailto:hlhr202@163.com"
            >
              My Email
            </a>
          </p>
        </div>
        <DialogFooter className="mx-0 mb-0 border-t px-6 py-3">
          <Button
            type="button"
            onClick={() => props.onOpenChange(false)}
            className="w-full"
          >
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
