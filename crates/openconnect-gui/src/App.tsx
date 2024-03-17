import {
  NextUIProvider,
  Button,
  Card,
  CardBody,
  CardFooter,
  CircularProgress,
  Link,
} from "@nextui-org/react";
import { TauriTitleBar } from "./Titlebar";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { atom, useAtom } from "jotai";
import { ServerSelector } from "./ServerSelector";
import bg from "./assets/bg.webp";
import { useStoredConfigs } from "./state";
import { handleError } from "./lib/error";
import { ToastContainer } from "react-toastify";
import connected from "./assets/connected-animate.json";
import Lottie from "lottie-react";
import { AboutModal } from "./About";

enum EStatus {
  Initialized = "INITIALIZED",
  Disconnecting = "DISCONNECTING",
  Disconnected = "DISCONNECTED",
  Connecting = "CONNECTING",
  Connected = "CONNECTED",
  Error = "ERROR",
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
  const [isAboutOpened, setIsAboutOpened] = useState(false);

  const handleConnect = useCallback(async () => {
    if (selectedServer) {
      try {
        switch (selectedServer.authType) {
          case "oidc":
            await invoke("connect_with_oidc", {
              serverName: selectedServer.name,
            });
            break;
          case "password":
            await invoke("connect_with_password", {
              serverName: selectedServer.name,
            });
            break;
        }
      } catch (e) {
        handleError(e);
      }
    }
  }, [selectedServer]);

  const handleDisconnect = async () => {
    try {
      await invoke("disconnect");
    } catch (e) {
      handleError(e);
    }
  };

  useEffect(() => {
    if (vpnStatus.status === EStatus.Error) {
      handleError({ code: "VPN_ERROR", message: vpnStatus.message ?? "" });
    }
  }, [vpnStatus.status, vpnStatus.message]);

  useEffect(() => {
    invoke("trigger_state_retrieve");
  }, []);

  return (
    <NextUIProvider>
      <ToastContainer
        className="min-w-[600px]"
        theme="dark"
        hideProgressBar={false}
        autoClose={5000}
      />
      <main className="dark border-none select-none text-foreground h-[100vh] flex justify-center flex-col items-center rounded-lg overflow-hidden">
        <div className="fixed z-[-5] overflow-hidden w-full h-full rounded-lg">
          <img
            src={bg}
            alt="bg"
            style={{ objectFit: "cover" }}
            className="relative h-full w-full scale-[120%] blur brightness-60 contrast-70"
          />
        </div>
        <TauriTitleBar />
        <h1 className="font-thin pb-8 text-3xl select-none cursor-default [text-shadow:_0px_0px_10px_rgb(0_0_0_/_90%)]">
          Openconnect RS
        </h1>
        <Card
          isBlurred
          shadow="lg"
          className="max-w-[800px] min-w-[600px] max-h-[800px] min-h-[400px] dark:bg-default-100/70"
        >
          <CardBody>
            {(() => {
              switch (vpnStatus.status) {
                case EStatus.Initialized:
                case EStatus.Disconnected:
                case EStatus.Error:
                  return <ServerSelector />;
                case EStatus.Connecting:
                  return (
                    <div className="flex flex-col w-full h-full items-center justify-center gap-5">
                      <div className="flex gap-5 items-center">
                        <CircularProgress color="success" />
                        <span>Connecting...</span>
                      </div>
                      {vpnStatus.message && <p>{vpnStatus.message}</p>}
                    </div>
                  );

                case EStatus.Disconnecting:
                  return (
                    <div className="flex w-full h-full items-center justify-center">
                      <div className="flex gap-5 items-center">
                        <CircularProgress color="success" />
                        <span>Disconnecting...</span>
                      </div>
                    </div>
                  );

                case EStatus.Connected:
                  return (
                    <div className="flex flex-col gap-5 w-full h-full items-center justify-center">
                      {/* thanks https://lottiefiles.com/animations/trusted-sites-animation-dkgE4eMn7w */}
                      <Lottie
                        animationData={connected}
                        style={{ width: "100px", height: "100px" }}
                      />
                      <div>Connected</div>
                    </div>
                  );
              }
            })()}
          </CardBody>
          <CardFooter className="justify-center">
            {vpnStatus.status === EStatus.Connected && (
              <Button
                color="primary"
                className="m-3 w-[50%]"
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
          Copyright @2024 hlhr202{" "}
        </div>
        <div className="font-thin mt-1 text-xs select-none cursor-pointer">
          <Link
            className="text-white underline text-xs"
            onClick={() => setIsAboutOpened(true)}
          >
            About this App
          </Link>
        </div>
        <AboutModal
          isOpen={isAboutOpened}
          onOpen={() => setIsAboutOpened(true)}
          onOpenChange={setIsAboutOpened}
        />
      </main>
    </NextUIProvider>
  );
}

export default App;
