import React from "react";

export class ErrorBoundary extends React.Component<{ children: React.ReactNode; fallback?: React.ReactNode }, { hasError: boolean }> {
  constructor(props: { children: React.ReactNode; fallback?: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }
  static getDerivedStateFromError() {
    return { hasError: true };
  }
  render() {
    if (this.state.hasError) {
      return this.props.fallback || <div className="p-4 text-text-danger text-sm">Something went wrong.</div>;
    }
    return this.props.children;
  }
}

export default ErrorBoundary;