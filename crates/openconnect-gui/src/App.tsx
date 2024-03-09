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

interface Inputs {
  server: string;
  username: string;
  password: string;
}

function App() {
  const [data, setData] = useLocalStorage("_openconnect_rs_", {
    server: "",
    username: "",
    password: "",
  });

  const { handleSubmit, register } = useForm<Inputs>({
    values: data,
  });

  const onSubmit: SubmitHandler<Inputs> = async (data) => {
    setData(data);
    const result = invoke("connect", data as unknown as Record<string, string>);
    console.log(result);
  };

  const handleDisconnect = () => {
    invoke("disconnect");
  };

  return (
    <NextUIProvider>
      <TauriTitleBar />
      <main className="select-none dark text-foreground bg-background h-[calc(100vh-30px)] flex justify-center flex-col items-center mt-[30px]">
        <h1 className="font-thin pb-8 text-3xl select-none cursor-pointer">
          Openconnect RS
        </h1>
        <Card className="max-w-[800px] min-w-[400px]">
          <CardBody>
            <form onSubmit={handleSubmit(onSubmit)}>
              <Input
                {...register("server", { required: true })}
                label="Server:"
                labelPlacement="outside"
                placeholder="https://"
                className="p-3"
                size="md"
                required
              ></Input>
              <Input
                {...register("username", { required: true })}
                label="Username:"
                labelPlacement="outside"
                placeholder="admin"
                className="p-3"
                size="md"
                required
              ></Input>
              <Input
                {...register("password", { required: true })}
                label="Password:"
                labelPlacement="outside"
                placeholder="password"
                className="p-3"
                type="password"
                size="md"
                required
              ></Input>

              <Divider className="mt-4"></Divider>
              <Button type="submit" color="primary" className="m-3">
                Connect
              </Button>
            </form>
            <Button color="primary" className="m-3" onClick={handleDisconnect}>Disconnect</Button>
          </CardBody>
        </Card>
        <div className="font-thin pt-8 text-xs select-none cursor-none">
          Source codes license - LGPL
        </div>
        <div className="font-thin pt-2 text-xs select-none cursor-none">
          Copyright @2024 hlhr202
        </div>
      </main>
    </NextUIProvider>
  );
}

export default App;
