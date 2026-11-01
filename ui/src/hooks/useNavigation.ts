import { useNavigate } from "react-router-dom";

export function useNavigation() {
  const navigate = useNavigate();
  return (view: string, state?: unknown) => {
    navigate(`/${view}`, { state });
  };
}

export default useNavigation;