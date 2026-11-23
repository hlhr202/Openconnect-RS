import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { atom, useAtom } from "jotai";
import { ToastContainer } from "react-toastify";
import Lottie from "lottie-react";
import { useKey } from "react-use";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardFooter } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { TauriTitleBar } from "./Titlebar";
import { ServerSelector } from "./ServerSelector";
import bg from "./assets/bg.jpeg";
import { useStoredConfigs } from "./state";
import { toastError, toastSuccess } from "./lib/toast";
import connected from "./assets/connected-animate.json";
import { AboutModal } from "./About";
import { CertPromptModal } from "./CertPromptModal";

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
  const unlistenPromise = listen<VpnStatus>("vpnStatus", (event) => {
    set(event.payload);
  });
  return () => {
    unlistenPromise.then((unlisten) => unlisten());
  };
};

function App() {
  const [vpnStatus] = useAtom(vpnStatusAtom);
  const { selectedServer } = useStoredConfigs();
  const [isAboutOpened, setIsAboutOpened] = useState(false);
  const [mounted, setMounted] = useState(false);

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
        toastError(e);
      }
    }
  }, [selectedServer]);

  const handleDisconnect = useCallback(async () => {
    try {
      await invoke("disconnect");
    } catch (e) {
      toastError(e);
    }
  }, []);

  useKey(
    "Enter",
    (evt) => {
      if (
        isAboutOpened ||
        document.querySelector("[role='dialog']") ||
        (evt.target instanceof HTMLElement &&
          evt.target.closest("input, textarea, select, button, [role='listbox']"))
      ) {
        return;
      }
      if (vpnStatus.status === EStatus.Connected) {
        handleDisconnect();
      } else if (selectedServer) {
        handleConnect();
      }
    },
    undefined,
    [isAboutOpened, vpnStatus, selectedServer, handleConnect, handleDisconnect]
  );

  useEffect(() => {
    if (vpnStatus.status === EStatus.Error) {
      toastError({ code: "VPN_ERROR", message: vpnStatus.message ?? "" });
    }
  }, [vpnStatus.status, vpnStatus.message]);

  useEffect(() => {
    invoke("trigger_state_retrieve");
    setMounted(true);

    const unlistenUri = listen<string>("oidcvpn-uri", (event) => {
      toastSuccess(`Received callback URI: ${event.payload}`);
    });
    return () => {
      unlistenUri.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <>
      <ToastContainer
        className="min-w-[600px]"
        theme="dark"
        hideProgressBar={false}
        autoClose={5000}
      />
      <main className="dark relative flex h-screen flex-col items-center justify-center overflow-hidden rounded-lg border-none text-foreground select-none">
        <div className="fixed inset-0 z-[-5] overflow-hidden rounded-lg">
          <img
            src={bg}
            alt="bg"
            style={{ objectFit: "cover" }}
            className={`h-full w-full scale-[120%] brightness-60 contrast-75 transition-all duration-[2000ms] ${
              mounted ? "blur-md" : "blur-xs"
            }`}
          />
        </div>
        <TauriTitleBar />
        <h1 className="cursor-default pb-8 text-3xl font-light tracking-tight select-none [text-shadow:_0px_0px_10px_rgb(0_0_0_/_90%)]">
          Openconnect RS
        </h1>
        <Card className="min-h-[400px] w-[620px] max-w-[800px] justify-between bg-card/75 shadow-2xl ring-white/10 backdrop-blur-xl">
          <CardContent className="flex flex-1 flex-col pt-1">
            {(() => {
              switch (vpnStatus.status) {
                case EStatus.Initialized:
                case EStatus.Disconnected:
                case EStatus.Error:
                  return <ServerSelector />;
                case EStatus.Connecting:
                  return (
                    <div className="flex h-full w-full flex-1 flex-col items-center justify-center gap-4">
                      <div className="flex items-center gap-3">
                        <Spinner className="size-6 text-emerald-400" />
                        <span className="text-base">Connecting...</span>
                      </div>
                      {vpnStatus.message && (
                        <p className="text-sm text-muted-foreground">
                          {vpnStatus.message}
                        </p>
                      )}
                    </div>
                  );

                case EStatus.Disconnecting:
                  return (
                    <div className="flex h-full w-full flex-1 items-center justify-center">
                      <div className="flex items-center gap-3">
                        <Spinner className="size-6 text-emerald-400" />
                        <span className="text-base">Disconnecting...</span>
                      </div>
                    </div>
                  );

                case EStatus.Connected:
                  return (
                    <div className="flex h-full w-full flex-1 flex-col items-center justify-center gap-4">
                      {/* thanks https://lottiefiles.com/animations/trusted-sites-animation-dkgE4eMn7w */}
                      <Lottie
                        animationData={connected}
                        style={{ width: "100px", height: "100px" }}
                      />
                      <div className="text-base font-medium">Connected</div>
                    </div>
                  );
              }
            })()}
          </CardContent>
          <CardFooter className="justify-center border-white/10 bg-black/15 py-4">
            {vpnStatus.status === EStatus.Connected && (
              <Button
                type="button"
                size="lg"
                className="w-1/2"
                onClick={handleDisconnect}
              >
                Disconnect [Enter]
              </Button>
            )}
            {vpnStatus.status === EStatus.Disconnected ||
            vpnStatus.status === EStatus.Initialized ||
            vpnStatus.status === EStatus.Error ? (
              <Button
                type="button"
                size="lg"
                className="w-full bg-emerald-600 text-white hover:bg-emerald-500"
                onClick={handleConnect}
              >
                Connect [Enter]
              </Button>
            ) : null}
          </CardFooter>
        </Card>
        <div className="mt-10 cursor-default text-xs font-light text-white/80 select-none">
          Source codes license - LGPL
        </div>
        <div className="mt-1 cursor-default text-xs font-light text-white/80 select-none">
          Copyright @2024 hlhr202{" "}
        </div>
        <div className="mt-1 text-xs font-light select-none">
          <button
            type="button"
            className="cursor-pointer rounded-xs text-xs text-white underline underline-offset-4 outline-none hover:text-white/80 focus-visible:ring-2 focus-visible:ring-ring/50"
            onClick={() => setIsAboutOpened(true)}
          >
            About this App
          </button>
        </div>
        <CertPromptModal />
        <AboutModal
          isOpen={isAboutOpened}
          onOpen={() => setIsAboutOpened(true)}
          onOpenChange={setIsAboutOpened}
        />
      </main>
    </>
  );
}

export default App;
