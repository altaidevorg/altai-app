export { OperationsContextBadge } from "./OperationsContextBadge";
export {
  OperationsStatusBar,
  recheckOperations,
  useOperationsProbe,
} from "./OperationsStatusBar";
export {
  OPERATIONS_CLIENT_NAME,
  OPERATIONS_PROTOCOL_CLIENT_VERSION,
  negotiateViaDesktop,
  operationsNegotiateParams,
  probeOperationsHealth,
  type NegotiateCommand,
  type OperationsHealth,
  type OperationsNegotiation,
} from "./lib/operationsHealth";
export {
  useOperationsContextStore,
  type OperationsConnection,
  type OperationsContext,
  type OperationsScope,
} from "./store/operationsContextStore";
