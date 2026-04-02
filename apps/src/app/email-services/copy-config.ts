import type { RegisterEmailService } from "@/types";

type EmailServiceCopyConfigPayload = Pick<
  RegisterEmailService,
  "id" | "name" | "serviceType" | "enabled" | "priority" | "config"
>;

export function buildEmailServiceCopyConfigJson(
  service: RegisterEmailService
): string {
  const payload: EmailServiceCopyConfigPayload = {
    id: service.id,
    name: service.name,
    serviceType: service.serviceType,
    enabled: service.enabled,
    priority: service.priority,
    config: service.config || {},
  };

  return JSON.stringify(payload, null, 2);
}
