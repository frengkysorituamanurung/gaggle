import React, { useCallback, useEffect, useRef, useState } from "react";
import { useLocation } from "react-router-dom";
import { motion } from "framer-motion";
import { PanelLeft } from "lucide-react";
import { Button } from "../ui/Button";
import { useNavigationContext } from "./NavigationContext";
import { Z_INDEX } from "./constants";
import { cn } from "../../lib/utils";

interface AppLayoutProps {
  sidebar: React.ReactNode;
  children?: React.ReactNode;
}

export const AppLayout: React.FC<AppLayoutProps> = ({ sidebar, children }) => {
  const location = useLocation();
  const isOnSettingsRoute = location.pathname === "/settings";

  const { isNavExpanded, setIsNavExpanded, navWidth, setNavWidth } = useNavigationContext();
  const [isDragging, setIsDragging] = useState(false);
  const isResizing = useRef(false);
  const startX = useRef(0);
  const startWidth = useRef(0);

  const handleResizeMouseDown = useCallback(
    (e: React.MouseEvent) => {
      isResizing.current = true;
      startX.current = e.clientX;
      startWidth.current = navWidth;
      setIsDragging(true);
      e.preventDefault();
    },
    [navWidth]
  );

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.current) return;
      setNavWidth(startWidth.current + (e.clientX - startX.current));
    };
    const handleMouseUp = () => {
      if (isResizing.current) {
        isResizing.current = false;
        setIsDragging(false);
      }
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [setNavWidth]);

  if (isOnSettingsRoute) {
    return (
      <div className="relative flex h-full w-full flex-1 bg-background-secondary">
        {children}
      </div>
    );
  }

  return (
    <div className="flex flex-1 w-full h-full relative animate-fade-in bg-background-primary flex-row">
      <div style={{ zIndex: Z_INDEX.HEADER }} className={cn("absolute flex items-center gap-1 pl-4 top-[11px] ml-1.5")}>
        <Button
          onClick={() => setIsNavExpanded(!isNavExpanded)}
          className="no-drag hover:!bg-background-tertiary"
          variant="ghost"
          size="sm"
          shape="round"
          title={isNavExpanded ? "Collapse navigation" : "Open navigation"}
          aria-label={isNavExpanded ? "Collapse navigation" : "Open navigation"}
        >
          <PanelLeft className="w-5 h-5" />
        </Button>
      </div>

      <div className="flex flex-1 w-full h-full min-h-0 flex-row">
        <motion.div
          key="nav"
          initial={false}
          animate={{ width: isNavExpanded ? navWidth : 0 }}
          transition={isDragging ? { duration: 0 } : { type: "spring", stiffness: 400, damping: 40 }}
          style={{ height: "100%" }}
          className="relative flex-shrink-0 overflow-hidden h-full p-2"
        >
          <div className="w-full h-full overflow-hidden rounded-xl border border-border-primary">
            {sidebar}
          </div>
          {isNavExpanded && (
            <div
              className="absolute right-0 top-0 h-full w-2 cursor-col-resize hover:bg-border-primary/30 transition-colors"
              onMouseDown={handleResizeMouseDown}
            />
          )}
        </motion.div>

        <div className="flex-1 overflow-hidden min-h-0">
          {children}
        </div>
      </div>
    </div>
  );
};