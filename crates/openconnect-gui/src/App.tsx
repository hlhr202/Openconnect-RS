import {
  NextUIProvider,
  Button,
  Card,
  CardBody,
  CardFooter,
} from "@nextui-org/react";
import { TauriTitleBar } from "./Titlebar";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useCallback } from "react";
import { atom, useAtom } from "jotai";
import { ServerSelector } from "./ServerSelector";
import bg from "./assets/bg.webp";
import { useStoredConfigs } from "./state";

enum EStatus {
  Initialized = "initialized",
  Disconnecting = "disconnecting",
  Disconnected = "disconnected",
  Connecting = "connecting",
  Connected = "connected",
  Error = "error",
}
interface VpnStatus {
  status: EStatus;
  message?: string;
}

const vpnStatusAtom = atom<VpnStatus>({ status: EStatus.Initialized });
vpnStatusAtom.onMount = (set) => {
  listen<VpnStatus>("vpnStatus", (event) => {
    set(event.payload);
  });
};

function App() {
  const [vpnStatus] = useAtom(vpnStatusAtom);
  const { selectedServer } = useStoredConfigs();

  const handleConnect = useCallback(() => {
    if (selectedServer) {
      switch (selectedServer.authType) {
        case "oidc":
          invoke("connect_with_oidc", { serverName: selectedServer.name });
          break;
        case "password":
          invoke("connect_with_password", { serverName: selectedServer.name });
          break;
      }
    }
  }, [selectedServer]);

  const handleDisconnect = () => {
    invoke("disconnect");
  };

  return (
    <NextUIProvider>
      <main className="dark border-none select-none text-foreground h-[100vh] flex justify-center flex-col items-center rounded-lg overflow-hidden">
        <div className="fixed z-[-5] overflow-hidden w-full h-full rounded-lg">
          <img
            src={bg}
            alt="bg"
            style={{ objectFit: "cover" }}
            className="relative h-full w-full scale-[120%] blur brightness-60"
          />
        </div>
        <TauriTitleBar />
        <h1 className="font-thin pb-8 text-3xl select-none cursor-default [text-shadow:_0px_0px_10px_rgb(0_0_0_/_90%)]">
          Openconnect RS
        </h1>
        <Card isBlurred shadow="lg" className="max-w-[800px] min-w-[600px] max-h-[800px] min-h-[400px] dark:bg-default-100/70">
          <CardBody>
            {(() => {
              switch (vpnStatus.status) {
                case EStatus.Initialized:
                case EStatus.Disconnected:
                case EStatus.Error:
                  return <ServerSelector />;
                case EStatus.Connecting:
                  return (
                    <div className="flex flex-col w-full h-full items-center justify-center">
                      <p>Connecting...</p>
                      {vpnStatus.message && <p>{vpnStatus.message}</p>}
                    </div>
                  );

                case EStatus.Disconnecting:
                  return (
                    <div className="flex w-full h-full items-center justify-center">
                      Disconnecting...
                    </div>
                  );

                case EStatus.Connected:
                  return (
                    <div className="flex w-full h-full items-center justify-center">
                      Connected
                    </div>
                  );
              }
            })()}
          </CardBody>
          <CardFooter>
            {vpnStatus.status === EStatus.Connected && (
              <Button
                color="primary"
                className="m-3 w-full"
                onClick={handleDisconnect}
              >
                Disconnect
              </Button>
            )}
            {vpnStatus.status === EStatus.Disconnected ||
            vpnStatus.status === EStatus.Initialized ||
            vpnStatus.status === EStatus.Error ? (
              <Button
                color="success"
                className="m-3 w-full"
                onClick={handleConnect}
              >
                Connect
              </Button>
            ) : null}
          </CardFooter>
        </Card>
        <div className="font-thin mt-10 text-xs select-none cursor-default">
          Source codes license - LGPL
        </div>
        <div className="font-thin mt-1 text-xs select-none cursor-default">
          Copyright @2024 hlhr202
        </div>
      </main>
    </NextUIProvider>
  );
}

export default App;
