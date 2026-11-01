import React from "react";
import { ResponseStyleSelectionItem } from "./ResponseStyleSelectionItem";
export function ResponseStylesSection() {
  const styles = ["Default", "Concise", "Detailed"];
  const [selected, setSelected] = React.useState("Default");
  return (
    <div className="space-y-2">
      <div className="font-medium text-text-primary">Response Styles</div>
      <div className="flex gap-2 flex-wrap">
        {styles.map((s) => (
          <ResponseStyleSelectionItem key={s} label={s} selected={selected === s} onClick={() => setSelected(s)} />
        ))}
      </div>
    </div>
  );
}
export default ResponseStylesSection;