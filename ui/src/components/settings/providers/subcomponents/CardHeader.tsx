import React from "react";
export function CardHeader({ children }: { children?: React.ReactNode }) { return <div className="font-medium text-text-primary">{children}</div>; }
export default CardHeader;