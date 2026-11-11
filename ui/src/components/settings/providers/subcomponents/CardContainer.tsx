import React from "react";
export function CardContainer({ children }: { children?: React.ReactNode }) { return <div className="rounded-md border border-border-primary bg-background-secondary p-3">{children}</div>; }
export default CardContainer;