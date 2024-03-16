import { Input, Button, Select, SelectItem } from "@nextui-org/react";
import {
  useForm,
  SubmitHandler,
  useWatch,
  Controller,
  FieldErrors,
} from "react-hook-form";
import { useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api";
import { OidcServer, PasswordServer, useStoredConfigs } from "./state";

export interface FormParams {
  mode: "add" | "edit";
  name?: string;
}

export const ServerEditor = (props: FormParams) => {
  const { getStoredConfigs, serverList } = useStoredConfigs();

  const initialData = useMemo(() => {
    const initial = {
      name: "",
      authType: undefined,
      server: "",
    };
    switch (props.mode) {
      case "add":
        return initial;
      case "edit":
        return (
          serverList.find((server) => server.name === props.name) ?? initial
        );
    }
  }, [props.name, props.mode, serverList]);

  const { handleSubmit, register, reset, unregister, control } = useForm<
    OidcServer | PasswordServer
  >({ defaultValues: initialData });

  const save: SubmitHandler<OidcServer | PasswordServer> = async (data) => {
    let toSave: OidcServer | PasswordServer;
    switch (data.authType) {
      case "oidc":
        toSave = {
          name: data.name,
          authType: "oidc",
          server: data.server,
          issuer: data.issuer,
          clientId: data.clientId,
          clientSecret: data.clientSecret,
        };
        break;
      case "password":
        toSave = {
          name: data.name,
          authType: "password",
          server: data.server,
          username: data.username,
          password: data.password,
        };
        break;
    }

    await invoke("upsert_stored_server", { server: toSave });
    await getStoredConfigs();
  };

  const watchedAuthType = useWatch({ control, name: "authType" });

  useEffect(() => {
    switch (initialData?.authType) {
      case "oidc":
        unregister("username");
        unregister("password");
        break;
      case "password":
        unregister("issuer");
        unregister("clientId");
        unregister("clientSecret");
        break;
    }
    reset(initialData);
  }, [initialData, register]);

  return (
    <form onSubmit={handleSubmit(save)} className="flex flex-col w-full gap-4">
      <Controller
        name="name"
        control={control}
        rules={{ required: "This field is required" }}
        render={({ field, formState }) => (
          <Input
            label="Name:"
            labelPlacement="inside"
            placeholder="My Server"
            size="sm"
            errorMessage={formState.errors.name?.message}
            {...field}
          />
        )}
      />

      <Controller
        name="authType"
        control={control}
        rules={{ required: "This field is required" }}
        render={({ field, formState }) => (
          <Select
            label="Authentication Type:"
            labelPlacement="inside"
            placeholder="Select an authentication type"
            selectionMode="single"
            unselectable="off"
            size="sm"
            errorMessage={formState.errors.authType?.message}
            selectedKeys={[field.value]}
            {...field}
          >
            <SelectItem key="oidc" value="oidc">
              OIDC Server
            </SelectItem>
            <SelectItem key="password" value="password">
              Password Server
            </SelectItem>
          </Select>
        )}
      />

      <Controller
        name="server"
        control={control}
        rules={{ required: "This field is required" }}
        render={({ field, formState }) => (
          <Input
            label="Server:"
            labelPlacement="inside"
            placeholder="https://"
            size="sm"
            errorMessage={formState.errors.server?.message}
            {...field}
          />
        )}
      />
      {watchedAuthType === "password" && (
        <>
          <Controller
            name="username"
            control={control}
            rules={{ required: "This field is required" }}
            render={({ field, formState }) => (
              <Input
                label="Username:"
                labelPlacement="inside"
                placeholder="username"
                size="sm"
                errorMessage={
                  (formState.errors as FieldErrors<PasswordServer>).username
                    ?.message
                }
                {...field}
              />
            )}
          />
          <Controller
            name="password"
            control={control}
            rules={{ required: "This field is required" }}
            render={({ field, formState }) => (
              <Input
                label="Password:"
                labelPlacement="inside"
                placeholder="password"
                size="sm"
                type="password"
                errorMessage={
                  (formState.errors as FieldErrors<PasswordServer>).password
                    ?.message
                }
                {...field}
              />
            )}
          />
        </>
      )}
      {watchedAuthType === "oidc" && (
        <>
          <Controller
            name="issuer"
            control={control}
            rules={{ required: "This field is required" }}
            render={({ field, formState }) => (
              <Input
                label="Issuer:"
                labelPlacement="inside"
                placeholder="https://"
                size="sm"
                errorMessage={
                  (formState.errors as FieldErrors<OidcServer>).issuer?.message
                }
                {...field}
              />
            )}
          />
          <Controller
            name="clientId"
            control={control}
            rules={{ required: "This field is required" }}
            render={({ field, formState }) => (
              <Input
                label="Client ID:"
                labelPlacement="inside"
                placeholder="client_id"
                size="sm"
                errorMessage={
                  (formState.errors as FieldErrors<OidcServer>).clientId
                    ?.message
                }
                {...field}
              />
            )}
          />
          <Controller
            name="clientSecret"
            control={control}
            render={({ field }) => (
              <Input
                label="Client Secret:"
                labelPlacement="inside"
                placeholder="client_secret"
                size="sm"
                {...field}
              />
            )}
          />
        </>
      )}
      <div className="flex gap-4 w-full">
        {props.mode === "edit" && (
          <Button type="button" color="danger" size="sm" className="flex-1">
            Delete
          </Button>
        )}
        <Button type="submit" color="primary" size="sm" className="flex-1">
          Save
        </Button>
      </div>
    </form>
  );
};
