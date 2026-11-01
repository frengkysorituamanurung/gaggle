import React from "react";
import { DefaultCardButtons } from "./DefaultCardButtons";
export function CardButtons(props: { onEdit?: () => void; onDelete?: () => void }) {
  return <DefaultCardButtons {...props} />;
}
export default CardButtons;