import { Input, Divider, Button } from "@nextui-org/react";
import { invoke } from "@tauri-apps/api";
import { useForm, SubmitHandler } from "react-hook-form";
import { useLocalStorage } from "react-use";

interface Inputs {
  server: string;
  username: string;
  password: string;
}

export const ServerEditor = () => {
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

  <form onSubmit={handleSubmit(onSubmit)} className="flex flex-col w-full">
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
  </form>;
};
