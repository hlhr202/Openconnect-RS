import {
  Input,
  Button,
  Select,
  SelectItem,
  Switch,
  cn,
} from "@heroui/react";
import { useForm, SubmitHandler, useWatch, Controller } from "react-hook-form";
import { useCallback, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OidcServer, PasswordServer, useStoredConfigs } from "./state";
import { toastError, toastSuccess } from "./lib/toast";
import { encodeUtf8Base64 } from "./lib/base64";

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
          allowInsecure: data.allowInsecure,
          serverCert: data.serverCert,
        };
        break;
      case "password":
        toSave = {
          name: data.name,
          authType: "password",
          server: data.server,
          username: data.username,
          password: data.password,
          allowInsecure: data.allowInsecure,
          serverCert: data.serverCert,
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
        const { updatedAt, name, serverCert, ...rest } = initialData;
        toShare = rest;
        break;
      }
      case "password": {
        const { password, username, updatedAt, name, serverCert, ...rest } =
          initialData;
        toShare = rest;
        break;
      }
    }
    const base64 = encodeUtf8Base64(JSON.stringify(toShare));
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
  }, [initialData, unregister, reset]);

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
              disallowEmptySelection
              size="sm"
              errorMessage={fieldState.error?.message}
              selectedKeys={field.value ? [field.value] : []}
              onSelectionChange={(keys) => {
                const value = Array.from(keys as Set<string>)[0];
                if (value) {
                  field.onChange(value);
                }
              }}
            >
              <SelectItem key="oidc">OIDC Server</SelectItem>
              <SelectItem key="password">Password Server</SelectItem>
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
        <Controller
          name="allowInsecure"
          control={control}
          render={({ field }) => (
            <Switch
              classNames={{
                base: cn(
                  "inline-flex flex-row-reverse w-full bg-content2 hover:bg-content3 items-center",
                  "justify-between cursor-pointer rounded-lg gap-2 p-2 border-2 border-transparent"
                ),
                wrapper: "p-0 h-4 overflow-visible",
                thumb: cn(
                  "w-6 h-6 border-2 shadow-lg",
                  "group-data-[selected=true]:ml-6",
                  "group-data-[pressed=true]:w-7",
                  "group-data-[selected]:group-data-[pressed]:ml-4"
                ),
              }}
              size="sm"
              aria-label="Allow Insecure SSL Certificate"
              isSelected={field.value ?? false}
              onValueChange={field.onChange}
            >
              <div className="flex flex-col gap-1">
                <p className="text-medium">Allow Insecure SSL Certificate</p>
                <p className="text-tiny text-default-400">
                  Advanced and unsafe: accept ANY server certificate without
                  asking. Prefer trusting the fingerprint when prompted on
                  connect.
                </p>
              </div>
            </Switch>
          )}
        />
        <Controller
          name="serverCert"
          control={control}
          render={({ field }) => (
            <div className="flex w-full items-center gap-2 rounded-lg bg-content2 p-2">
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <p className="text-medium">Trusted Server Certificate</p>
                <p className="text-tiny text-default-400 break-all font-mono">
                  {field.value
                    ? field.value
                    : "None. You will be asked to verify the fingerprint if the certificate cannot be validated by a trusted authority."}
                </p>
              </div>
              {field.value && (
                <Button
                  type="button"
                  size="sm"
                  color="danger"
                  variant="flat"
                  onPress={() => field.onChange("")}
                >
                  Forget
                </Button>
              )}
            </div>
          )}
        />
      </div>
      <div className="flex gap-4 w-full self-end items-end pl-2 pr-2">
        {props.mode === "edit" && (
          <>
            <Button
              type="button"
              color="warning"
              size="sm"
              className="flex-1"
              onPress={handleShare}
            >
              Share
            </Button>
            <Button
              type="button"
              color="danger"
              size="sm"
              className="flex-1"
              isDisabled={defaultName === initialData.name}
              onPress={removeServer}
            >
              Delete
            </Button>
            <Button
              type="button"
              color="primary"
              size="sm"
              className="flex-1"
              isDisabled={defaultName === initialData.name}
              onPress={setDefaultServer}
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
