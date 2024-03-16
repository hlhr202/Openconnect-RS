import { NextUIProvider, Button, Card, CardBody } from "@nextui-org/react";
import { TauriTitleBar } from "./Titlebar";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect } from "react";
import { atom, useAtom } from "jotai";
import { ServerSelector } from "./ServerSelector";
import bg from "./assets/bg.webp";
import { OidcServer, PasswordServer } from "./state";

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

  const handleConnect = useCallback((server: OidcServer | PasswordServer) => {
    if (server.authType === "oidc") {
      invoke("connect_with_oidc", { server_name: server.name });
    } else {
      invoke("connect_with_password", { server_name: server.name });
    }
  }, []);

  const handleDisconnect = () => {
    invoke("disconnect");
  };

  useEffect(() => {
    console.log(vpnStatus);
  }, [vpnStatus]);

  return (
    <NextUIProvider>
      <main className="dark border-none select-none text-foreground h-[100vh] flex justify-center flex-col items-center">
        <div className="fixed z-[-5] overflow-hidden w-full h-full rounded-lg">
          <img
            src={bg}
            alt="bg"
            style={{ objectFit: "cover" }}
            className="relative h-full w-full scale-[120%] blur-sm brightness-60"
          />
        </div>
        <TauriTitleBar />
        <h1 className="font-thin pb-8 text-3xl select-none cursor-default z-[1]">
          Openconnect RS
        </h1>
        <Card className="max-w-[800px] min-w-[600px] max-h-[800px] min-h-[400px] bg-opacity-85">
          <CardBody>
            {(() => {
              switch (vpnStatus.status) {
                case EStatus.Initialized:
                case EStatus.Disconnected:
                case EStatus.Error:
                  return <ServerSelector onConnect={handleConnect} />;
                case EStatus.Connecting:
                  return (
                    <div className="flex w-full h-full items-center justify-center">
                      Connecting...
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
                      <Button
                        color="primary"
                        className="m-3"
                        onClick={handleDisconnect}
                      >
                        Disconnect
                      </Button>
                    </div>
                  );
              }
            })()}
          </CardBody>
        </Card>
        <div className="font-thin pt-8 text-xs select-none cursor-default">
          Source codes license - LGPL
        </div>
        <div className="font-thin pt-2 text-xs select-none cursor-default">
          Copyright @2024 hlhr202
        </div>
      </main>
    </NextUIProvider>
  );
}

export default App;
