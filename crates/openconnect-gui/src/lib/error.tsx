import { toast } from "react-toastify";

interface IError {
  code: string;
  message: string;
}

export const handleError = (e: unknown) => {
  const error = e as IError;
  console.log(e)

  toast.error(
    <div className="flex flex-col gap-4 w-full">
      <h3>Error Code: {error.code}</h3>
      <p className="break-words">Error Message: {error.message}</p>
    </div>,
    { autoClose: 10000, position: "bottom-center" }
  );
};
