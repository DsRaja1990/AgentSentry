# Reference Rego policy used by the demo (seeded automatically into the
# control plane by the dev stack; included here for clarity).
package agentsentry.tool_call

default decision := {"allow": true}

decision := {
    "allow":      false,
    "reason":     "external email recipient",
    "policy_id":  "pol_block_external_email",
    "obligations": ["log_to_audit"]
} if {
    input.tool.name == "send_email"
    not endswith(input.tool.args.to, "@contoso.com")
}
