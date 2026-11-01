import React from "react";
export function DefaultSubmitHandler({ onSubmit }: { onSubmit?: () => void }) {
  return <button onClick={onSubmit} className="hidden" />;
}
export default DefaultSubmitHandler;