import React from "react";

export function FlyingBird() {
  return (
    <div className="inline-block animate-bounce motion-reduce:animate-none" aria-hidden="true">🐦</div>
  );
}

export default FlyingBird;