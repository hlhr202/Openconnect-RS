import { FC, PropsWithChildren } from "react";
import { appWindow } from "@tauri-apps/api/window";

const TitleBarButton: FC<
  PropsWithChildren<{ id: string; className?: string; onClick?: () => void }>
> = ({ onClick, className, children, id }) => {
  return (
    <div
      onClick={onClick}
      id={id}
      className={`${className} inline-flex justify-center items-center w-[45px] h-[38px] hover:bg-gray-600 transition-colors`}
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
      className="titlebar dark z-[51] h-[15vh] bg-gradient-to-b from-gray-900 to-transparent select-none flex justify-end fixed top-0 left-0 right-0 rounded-t-lg"
    >
      <TitleBarButton id="titlebar-minimize" onClick={minimize}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="1.2em"
          height="1.2em"
          viewBox="0 0 24 24"
        >
          <path fill="white" d="M20 14H4v-4h16" />
        </svg>
      </TitleBarButton>
      <TitleBarButton id="titlebar-maximize" onClick={maximize}>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="1.2em"
          height="1.2em"
          viewBox="0 0 24 24"
        >
          <path fill="white" d="M4 4h16v16H4zm2 4v10h12V8z" />
        </svg>
      </TitleBarButton>
      <TitleBarButton
        id="titlebar-close"
        onClick={close}
        className="rounded-tr-lg"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="1.2em"
          height="1.2em"
          viewBox="0 0 24 24"
        >
          <path
            fill="white"
            d="M13.46 12L19 17.54V19h-1.46L12 13.46L6.46 19H5v-1.46L10.54 12L5 6.46V5h1.46L12 10.54L17.54 5H19v1.46z"
          />
        </svg>
      </TitleBarButton>
    </div>
  );
};
