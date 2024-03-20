import { Input, Button, Select, SelectItem } from "@nextui-org/react";
import { useForm, SubmitHandler, useWatch, Controller } from "react-hook-form";
import { useCallback, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api";
import { OidcServer, PasswordServer, useStoredConfigs } from "./state";
import { toastError, toastSuccess } from "./lib/toast";
import { enc } from "crypto-js";

export interface FormParams {
  mode: "add" | "edit";
  name?: string;
  addFromImport?: Partial<OidcServer | PasswordServer>;
}

export const ServerEditor = (props: FormParams) => {
  const { getStoredConfigs, serverList, defaultName } = useStoredConfigs();

  const initialData = useMemo(() => {
    const empty = {
      name: "",
      authType: undefined,
      server: "",
    };
    switch (props.mode) {
      case "add": {
        return props.addFromImport ?? empty;
      }
      case "edit":
        return serverList.find((server) => server.name === props.name) ?? empty;
    }
  }, [props.name, props.mode, props.addFromImport, serverList]);

  const { handleSubmit, reset, unregister, control } = useForm<
    OidcServer | PasswordServer
  >();

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

    try {
      await invoke("upsert_stored_server", { server: toSave });
      await getStoredConfigs();
      toastSuccess("Saved server successfully");
    } catch (e) {
      toastError(e);
    }
  };

  const setDefaultServer = useCallback(async () => {
    try {
      await invoke("set_default_server", { serverName: initialData.name });
      await getStoredConfigs();
      toastSuccess("Set default server successfully");
    } catch (e) {
      toastError(e);
    }
  }, [initialData.name, getStoredConfigs]);

  const removeServer = useCallback(async () => {
    try {
      await invoke("remove_server", { serverName: initialData.name });
      await getStoredConfigs();
      toastSuccess("Removed server successfully");
    } catch (e) {
      toastError(e);
    }
  }, [initialData.name, getStoredConfigs]);

  const watchedAuthType = useWatch({ control, name: "authType" });

  const handleShare = useCallback(() => {
    let toShare: Partial<OidcServer | PasswordServer> = {};
    switch (initialData.authType) {
      case "oidc": {
        const { updatedAt, name, ...rest } = initialData;
        toShare = rest;
        break;
      }
      case "password": {
        const { password, username, updatedAt, name, ...rest } = initialData;
        toShare = rest;
        break;
      }
    }
    const jsonString = JSON.stringify(toShare);
    const words = enc.Utf8.parse(jsonString);
    const base64 = enc.Base64.stringify(words);
    navigator.clipboard.writeText(base64);
    toastSuccess("Copied to clipboard successfully");
  }, [initialData]);

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
    reset(initialData, {
      keepValues: false,
      keepDefaultValues: false,
      keepDirty: false,
      keepErrors: false,
      keepDirtyValues: false,
      keepTouched: false,
    });
  }, [initialData, unregister]);

  return (
    <form
      onSubmit={handleSubmit(save)}
      className="flex flex-col w-full gap-4 h-full"
    >
      <div className="flex flex-col w-full gap-4 h-[350px] overflow-auto p-2">
        <Controller
          name="name"
          control={control}
          rules={{ required: "This field is required" }}
          render={({ field, fieldState }) => (
            <Input
              label="Name:"
              labelPlacement="inside"
              placeholder="My Server"
              size="sm"
              isDisabled={props.mode === "edit"}
              errorMessage={fieldState.error?.message}
              {...field}
            />
          )}
        />

        <Controller
          name="authType"
          control={control}
          rules={{ required: "This field is required" }}
          render={({ field, fieldState }) => (
            <Select
              label="Authentication Type:"
              labelPlacement="inside"
              placeholder="Select an authentication type"
              selectionMode="single"
              unselectable="off"
              disallowEmptySelection
              size="sm"
              errorMessage={fieldState.error?.message}
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
          render={({ field, fieldState }) => (
            <Input
              label="Server:"
              labelPlacement="inside"
              placeholder="https://"
              size="sm"
              errorMessage={fieldState.error?.message}
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
              render={({ field, fieldState }) => (
                <Input
                  label="Username:"
                  labelPlacement="inside"
                  placeholder="username"
                  size="sm"
                  errorMessage={fieldState.error?.message}
                  {...field}
                />
              )}
            />
            <Controller
              name="password"
              control={control}
              rules={{ required: "This field is required" }}
              render={({ field, fieldState }) => (
                <Input
                  label="Password:"
                  labelPlacement="inside"
                  placeholder="password"
                  size="sm"
                  type="password"
                  errorMessage={fieldState.error?.message}
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
              render={({ field, fieldState }) => (
                <Input
                  label="Issuer:"
                  labelPlacement="inside"
                  placeholder="https://"
                  size="sm"
                  errorMessage={fieldState.error?.message}
                  {...field}
                />
              )}
            />
            <Controller
              name="clientId"
              control={control}
              rules={{ required: "This field is required" }}
              render={({ field, fieldState }) => (
                <Input
                  label="Client ID:"
                  labelPlacement="inside"
                  placeholder="client_id"
                  size="sm"
                  errorMessage={fieldState.error?.message}
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
      </div>
      <div className="flex gap-4 w-full self-end items-end pl-2 pr-2">
        {props.mode === "edit" && (
          <>
            <Button
              type="button"
              color="warning"
              size="sm"
              className="flex-1"
              onClick={handleShare}
            >
              Share
            </Button>
            <Button
              type="button"
              color="danger"
              size="sm"
              className="flex-1"
              disabled={defaultName === initialData.name}
              isDisabled={defaultName === initialData.name}
              onClick={removeServer}
            >
              Delete
            </Button>
            <Button
              type="button"
              color="primary"
              size="sm"
              className="flex-1"
              disabled={defaultName === initialData.name}
              isDisabled={defaultName === initialData.name}
              onClick={setDefaultServer}
            >
              Set Default
            </Button>
          </>
        )}
        <Button type="submit" color="success" size="sm" className="flex-1">
          Save
        </Button>
      </div>
    </form>
  );
};
