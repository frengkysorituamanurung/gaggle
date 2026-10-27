import React from "react";
export default function ImagePreview({ src }: { src: string }) {
  return <img src={src} alt="preview" className="max-w-full rounded-md border border-border-primary" />;
}