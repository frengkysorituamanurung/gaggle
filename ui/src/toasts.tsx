import React from "react";
import { toast } from "react-toastify";

export function toastSuccess(msg: string) { toast.success(msg); }
export function toastError(msg: string) { toast.error(msg); }
export function toastInfo(msg: string) { toast.info(msg); }

export default { toastSuccess, toastError, toastInfo };