import React from "react";
import type * as T from "../../types";

export function RecipeHeader({ recipe }: { recipe: T.Recipe | null }) {
  if (!recipe) return null;
  return (
    <div className="mb-4">
      <h2 className="text-xl font-semibold text-text-primary">{recipe.title}</h2>
      {recipe.description && <p className="text-sm text-text-secondary mt-1">{recipe.description}</p>}
      <div className="text-xs text-text-tertiary mt-1">{recipe.name}</div>
    </div>
  );
}

export default RecipeHeader;