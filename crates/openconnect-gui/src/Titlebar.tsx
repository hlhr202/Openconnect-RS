import { FC, PropsWithChildren } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hide } from "@tauri-apps/api/app";
import { type as getOsType } from "@tauri-apps/plugin-os";
import { Minus, Square, X } from "lucide-react";
import { cn } from "@/lib/utils";

const TitleBarButton: FC<
  PropsWithChildren<{
    id: string;
    label: string;
    className?: string;
    onClick?: () => void;
  }>
> = ({ onClick, className, children, id, label }) => {
  return (
    <button
      type="button"
      onClick={onClick}
      id={id}
      aria-label={label}
      className={cn(
        "inline-flex h-10 w-11 items-center justify-center text-white/80 transition-colors outline-none hover:bg-white/10 hover:text-white focus-visible:bg-white/15 focus-visible:ring-2 focus-visible:ring-ring/50",
        className
      )}
    >
      {children}
    </button>
  );
};

export const TauriTitleBar = () => {
  const minimize = () => {
    getCurrentWindow().minimize();
  };

  const maximize = async () => {
    const appWindow = getCurrentWindow();
    if (await appWindow.isMaximized()) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
  };

  const handleHide = async () => {
    const osType = getOsType();
    if (osType === "macos") {
      await hide();
    } else {
      await getCurrentWindow().hide();
    }
  };

  return (
    <div
      data-tauri-drag-region
      className="titlebar fixed top-0 right-0 left-0 z-[60] flex h-10 select-none items-center justify-end rounded-t-lg bg-gradient-to-b from-black/45 to-transparent"
    >
      <TitleBarButton
        id="titlebar-minimize"
        label="Minimize window"
        onClick={minimize}
      >
        <Minus className="size-4" />
      </TitleBarButton>
      <TitleBarButton
        id="titlebar-maximize"
        label="Maximize window"
        onClick={maximize}
      >
        <Square className="size-3.5" />
      </TitleBarButton>
      <TitleBarButton
        id="titlebar-hide"
        label="Hide window"
        onClick={handleHide}
        className="rounded-tr-lg hover:bg-destructive/80"
      >
        <X className="size-4" />
      </TitleBarButton>
    </div>
  );
};
