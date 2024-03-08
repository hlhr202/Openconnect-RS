import { FC, PropsWithChildren } from "react";
import { appWindow } from "@tauri-apps/api/window";

const TitleBarButton: FC<
  PropsWithChildren<{ id: string; className?: string; onClick?: () => void }>
> = ({ onClick, className, children, id }) => {
  return (
    <div
      onClick={onClick}
      id={id}
      className={`${className} inline-flex justify-center items-center w-[30px] h-[30px] hover:bg-[#888]`}
    >
      {children}
    </div>
  );
};

export const TauriTitleBar = () => {
  const minimize = () => {
    appWindow.minimize();
  };

  const maximize = async () => {
    if (await appWindow.isMaximized()) appWindow.unmaximize();
    else appWindow.maximize();
  };

  const close = () => {
    appWindow.close();
  };

  return (
    <div
      data-tauri-drag-region
      className="titlebar dark h-[30px] bg-[#111] to-background select-none flex justify-end fixed top-0 left-0 right-0 pr-[5px] rounded-t-md"
    >
      <TitleBarButton id="titlebar-minimize" onClick={minimize}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="1em"
          height="1em"
          viewBox="0 0 512 512"
        >
          <path
            fill="white"
            d="M480 480H32c-17.7 0-32-14.3-32-32s14.3-32 32-32h448c17.7 0 32 14.3 32 32s-14.3 32-32 32"
          />
        </svg>
      </TitleBarButton>
      <TitleBarButton id="titlebar-maximize" onClick={maximize}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="1em"
          height="1em"
          viewBox="0 0 512 512"
        >
          <path
            fill="white"
            d="M464 32H48C21.5 32 0 53.5 0 80v352c0 26.5 21.5 48 48 48h416c26.5 0 48-21.5 48-48V80c0-26.5-21.5-48-48-48m0 394c0 3.3-2.7 6-6 6H54c-3.3 0-6-2.7-6-6V192h416z"
          />
        </svg>
      </TitleBarButton>
      <TitleBarButton id="titlebar-close" onClick={close}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="1em"
          height="1em"
          viewBox="0 0 512 512"
        >
          <path
            fill="white"
            d="M464 32H48C21.5 32 0 53.5 0 80v352c0 26.5 21.5 48 48 48h416c26.5 0 48-21.5 48-48V80c0-26.5-21.5-48-48-48m0 394c0 3.3-2.7 6-6 6H54c-3.3 0-6-2.7-6-6V86c0-3.3 2.7-6 6-6h404c3.3 0 6 2.7 6 6zM356.5 194.6L295.1 256l61.4 61.4c4.6 4.6 4.6 12.1 0 16.8l-22.3 22.3c-4.6 4.6-12.1 4.6-16.8 0L256 295.1l-61.4 61.4c-4.6 4.6-12.1 4.6-16.8 0l-22.3-22.3c-4.6-4.6-4.6-12.1 0-16.8l61.4-61.4l-61.4-61.4c-4.6-4.6-4.6-12.1 0-16.8l22.3-22.3c4.6-4.6 12.1-4.6 16.8 0l61.4 61.4l61.4-61.4c4.6-4.6 12.1-4.6 16.8 0l22.3 22.3c4.7 4.6 4.7 12.1 0 16.8"
          />
        </svg>
      </TitleBarButton>
    </div>
  );
};
