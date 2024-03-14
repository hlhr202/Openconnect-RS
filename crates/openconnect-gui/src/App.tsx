import {
  NextUIProvider,
  Button,
  Input,
  Card,
  CardBody,
  Divider,
} from "@nextui-org/react";
import { TauriTitleBar } from "./Titlebar";
import { useForm, SubmitHandler } from "react-hook-form";
import { useLocalStorage } from "react-use";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { atom, useAtom } from "jotai";

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

interface Inputs {
  server: string;
  username: string;
  password: string;
}

function App() {
  const [vpnStatus] = useAtom(vpnStatusAtom);

  const [data, setData] = useLocalStorage("_openconnect_rs_", {
    server: "",
    username: "",
    password: "",
  });

  const { handleSubmit, register, getValues } = useForm<Inputs>({
    defaultValues: data,
  });

  const onSubmit: SubmitHandler<Inputs> = async (data) => {
    setData(data);
    const result = invoke("connect", data as unknown as Record<string, string>);
    console.log(result);
  };

  const handleDisconnect = () => {
    invoke("disconnect");
  };

  const handleSSO = () => {
    invoke("connect_with_oidc", { server: data?.server });
  };

  useEffect(() => {
    console.log(vpnStatus);
  }, [vpnStatus]);

  return (
    <NextUIProvider>
      <TauriTitleBar />
      <main className="border-none select-none dark text-foreground bg-background h-[calc(100vh-30px)] flex justify-center flex-col items-center mt-[30px]">
        <h1 className="font-thin pb-8 text-3xl select-none cursor-default">
          Openconnect RS
        </h1>
        <Card className="max-w-[800px] min-w-[400px] max-h-[800px] min-h-[400px]">
          <CardBody>
            {(() => {
              switch (vpnStatus.status) {
                case EStatus.Initialized:
                case EStatus.Disconnected:
                case EStatus.Error:
                  return (
                    <>
                      <form
                        onSubmit={handleSubmit(onSubmit)}
                        className="flex flex-col w-full"
                      >
                        <Input
                          label="Server:"
                          labelPlacement="outside"
                          placeholder="https://"
                          className="p-3"
                          size="md"
                          defaultValue={getValues("server")}
                          {...register("server", { required: true })}
                        />
                        <Input
                          label="Username:"
                          labelPlacement="outside"
                          placeholder="admin"
                          className="p-3"
                          size="md"
                          defaultValue={getValues("username")}
                          {...register("username", { required: true })}
                        />
                        <Input
                          label="Password:"
                          labelPlacement="outside"
                          placeholder="password"
                          className="p-3"
                          type="password"
                          size="md"
                          defaultValue={getValues("password")}
                          {...register("password", { required: true })}
                        />

                        <Divider className="mt-4"></Divider>
                        <Button type="submit" color="primary" className="m-3">
                          Connect
                        </Button>
                      </form>
                      <Button
                        onClick={handleSSO}
                        color="primary"
                        className="m-3"
                      >
                        SSO
                      </Button>
                    </>
                  );
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
