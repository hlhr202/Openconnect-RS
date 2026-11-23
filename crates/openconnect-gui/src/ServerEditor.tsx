import { useForm, SubmitHandler, useWatch, Controller } from "react-hook-form";
import { useCallback, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
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
      className="flex h-full min-h-0 w-full flex-col gap-3"
    >
      <div className="flex min-h-0 flex-1 flex-col gap-3.5 overflow-y-auto px-1 py-0.5">
        <Controller
          name="name"
          control={control}
          rules={{ required: "This field is required" }}
          render={({ field, fieldState }) => (
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="server-name" className="text-xs text-muted-foreground">
                Name
              </Label>
              <Input
                id="server-name"
                placeholder="My Server"
                disabled={props.mode === "edit"}
                aria-invalid={fieldState.invalid}
                {...field}
                value={field.value ?? ""}
              />
              {fieldState.error && (
                <p className="text-xs text-destructive">
                  {fieldState.error.message}
                </p>
              )}
            </div>
          )}
        />

        <Controller
          name="authType"
          control={control}
          rules={{ required: "This field is required" }}
          render={({ field, fieldState }) => (
            <div className="flex flex-col gap-1.5">
              <Label
                htmlFor="server-auth-type"
                className="text-xs text-muted-foreground"
              >
                Authentication Type
              </Label>
              <Select
                value={field.value ?? ""}
                onValueChange={(value) => {
                  if (value) {
                    field.onChange(value);
                  }
                }}
              >
                <SelectTrigger
                  id="server-auth-type"
                  className="w-full"
                  aria-invalid={fieldState.invalid}
                >
                  <SelectValue placeholder="Select an authentication type" />
                </SelectTrigger>
                <SelectContent position="popper">
                  <SelectItem value="oidc">OIDC Server</SelectItem>
                  <SelectItem value="password">Password Server</SelectItem>
                </SelectContent>
              </Select>
              {fieldState.error && (
                <p className="text-xs text-destructive">
                  {fieldState.error.message}
                </p>
              )}
            </div>
          )}
        />

        <Controller
          name="server"
          control={control}
          rules={{ required: "This field is required" }}
          render={({ field, fieldState }) => (
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="server-url" className="text-xs text-muted-foreground">
                Server
              </Label>
              <Input
                id="server-url"
                placeholder="https://"
                spellCheck={false}
                aria-invalid={fieldState.invalid}
                {...field}
                value={field.value ?? ""}
              />
              {fieldState.error && (
                <p className="text-xs text-destructive">
                  {fieldState.error.message}
                </p>
              )}
            </div>
          )}
        />

        {watchedAuthType === "password" && (
          <>
            <Controller
              name="username"
              control={control}
              rules={{ required: "This field is required" }}
              render={({ field, fieldState }) => (
                <div className="flex flex-col gap-1.5">
                  <Label
                    htmlFor="server-username"
                    className="text-xs text-muted-foreground"
                  >
                    Username
                  </Label>
                  <Input
                    id="server-username"
                    placeholder="username"
                    spellCheck={false}
                    aria-invalid={fieldState.invalid}
                    {...field}
                    value={field.value ?? ""}
                  />
                  {fieldState.error && (
                    <p className="text-xs text-destructive">
                      {fieldState.error.message}
                    </p>
                  )}
                </div>
              )}
            />
            <Controller
              name="password"
              control={control}
              rules={{ required: "This field is required" }}
              render={({ field, fieldState }) => (
                <div className="flex flex-col gap-1.5">
                  <Label
                    htmlFor="server-password"
                    className="text-xs text-muted-foreground"
                  >
                    Password
                  </Label>
                  <Input
                    id="server-password"
                    type="password"
                    placeholder="password"
                    aria-invalid={fieldState.invalid}
                    {...field}
                    value={field.value ?? ""}
                  />
                  {fieldState.error && (
                    <p className="text-xs text-destructive">
                      {fieldState.error.message}
                    </p>
                  )}
                </div>
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
                <div className="flex flex-col gap-1.5">
                  <Label
                    htmlFor="server-issuer"
                    className="text-xs text-muted-foreground"
                  >
                    Issuer
                  </Label>
                  <Input
                    id="server-issuer"
                    placeholder="https://"
                    spellCheck={false}
                    aria-invalid={fieldState.invalid}
                    {...field}
                    value={field.value ?? ""}
                  />
                  {fieldState.error && (
                    <p className="text-xs text-destructive">
                      {fieldState.error.message}
                    </p>
                  )}
                </div>
              )}
            />
            <Controller
              name="clientId"
              control={control}
              rules={{ required: "This field is required" }}
              render={({ field, fieldState }) => (
                <div className="flex flex-col gap-1.5">
                  <Label
                    htmlFor="server-client-id"
                    className="text-xs text-muted-foreground"
                  >
                    Client ID
                  </Label>
                  <Input
                    id="server-client-id"
                    placeholder="client_id"
                    spellCheck={false}
                    aria-invalid={fieldState.invalid}
                    {...field}
                    value={field.value ?? ""}
                  />
                  {fieldState.error && (
                    <p className="text-xs text-destructive">
                      {fieldState.error.message}
                    </p>
                  )}
                </div>
              )}
            />
            <Controller
              name="clientSecret"
              control={control}
              render={({ field }) => (
                <div className="flex flex-col gap-1.5">
                  <Label
                    htmlFor="server-client-secret"
                    className="text-xs text-muted-foreground"
                  >
                    Client Secret
                  </Label>
                  <Input
                    id="server-client-secret"
                    placeholder="client_secret"
                    spellCheck={false}
                    {...field}
                    value={field.value ?? ""}
                  />
                </div>
              )}
            />
          </>
        )}

        <Controller
          name="allowInsecure"
          control={control}
          render={({ field }) => (
            <label
              htmlFor="server-allow-insecure"
              className="flex w-full cursor-pointer items-center justify-between gap-4 rounded-lg border bg-muted/40 p-3 transition-colors hover:bg-muted/60"
            >
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <span className="text-sm font-medium text-foreground">
                  Allow Insecure SSL Certificate
                </span>
                <span className="text-xs leading-relaxed text-muted-foreground">
                  Advanced and unsafe: accept ANY server certificate without
                  asking. Prefer trusting the fingerprint when prompted on
                  connect.
                </span>
              </div>
              <Switch
                id="server-allow-insecure"
                aria-label="Allow Insecure SSL Certificate"
                checked={field.value ?? false}
                onCheckedChange={field.onChange}
              />
            </label>
          )}
        />

        <Controller
          name="serverCert"
          control={control}
          render={({ field }) => (
            <div className="flex w-full items-center gap-3 rounded-lg border bg-muted/40 p-3">
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <p className="text-sm font-medium text-foreground">
                  Trusted Server Certificate
                </p>
                <p className="font-mono text-xs break-all text-muted-foreground">
                  {field.value
                    ? field.value
                    : "None. You will be asked to verify the fingerprint if the certificate cannot be validated by a trusted authority."}
                </p>
              </div>
              {field.value && (
                <Button
                  type="button"
                  size="sm"
                  variant="destructive"
                  onClick={() => field.onChange("")}
                >
                  Forget
                </Button>
              )}
            </div>
          )}
        />
      </div>

      <div className="flex w-full shrink-0 gap-2 pt-1">
        {props.mode === "edit" && (
          <>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="flex-1 border-amber-500/40 text-amber-400 hover:bg-amber-500/15 hover:text-amber-300"
              onClick={handleShare}
            >
              Share
            </Button>
            <Button
              type="button"
              variant="destructive"
              size="sm"
              className="flex-1"
              disabled={defaultName === initialData.name}
              onClick={removeServer}
            >
              Delete
            </Button>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="flex-1"
              disabled={defaultName === initialData.name}
              onClick={setDefaultServer}
            >
              Set Default
            </Button>
          </>
        )}
        <Button
          type="submit"
          size="sm"
          className="flex-1 bg-emerald-600 text-white hover:bg-emerald-500"
        >
          Save
        </Button>
      </div>
    </form>
  );
};
