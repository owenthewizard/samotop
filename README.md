# Samotop
An SMTP [server](samotop-server/README.md) and [library](samotop/README.md) implemented in Rust.

[![plantuml]](http://www.plantuml.com/plantuml/uml/TL5TIyCm57tFhxZiFGDzB28hAY9OsEnyAdl8sZiRbYOZ7-gG_NVJf5Sgubivf-VZdgoS5zQ7GR5EUB4N3c6n2HXm0L_iCWFBjZL1UvVnfghB7V1mub27_I2TaqP7T4le2ofnPiscsh7cCGZRHLpXmNEuwx4zCiQqwQ9j9QJQcqzmP-Tn6Cq1cWdyaToZakzepxyvAz_wI8v0RCHQPK87Kdkq6dqsAhNnFLgeqKPzrC24v7gNIHGSsYdvjToDPka3EB-TdLV0GrrjrsmCX1lEyzy5F5NbcWRfp8UE8XghWAibYE1xuiTx8fyMBk1w2SuRYt2G2cczR95dIlhd64falla_0nfwNi06XIuz15cJ-2JROnOcbhhD5mAwEVz1wVGUHUxsPsOAIXvaiKBTXK5z0m00)

[plantuml]: http://www.plantuml.com/plantuml/png/TL5TIyCm57tFhxZiFGDzB28hAY9OsEnyAdl8sZiRbYOZ7-gG_NVJf5Sgubivf-VZdgoS5zQ7GR5EUB4N3c6n2HXm0L_iCWFBjZL1UvVnfghB7V1mub27_I2TaqP7T4le2ofnPiscsh7cCGZRHLpXmNEuwx4zCiQqwQ9j9QJQcqzmP-Tn6Cq1cWdyaToZakzepxyvAz_wI8v0RCHQPK87Kdkq6dqsAhNnFLgeqKPzrC24v7gNIHGSsYdvjToDPka3EB-TdLV0GrrjrsmCX1lEyzy5F5NbcWRfp8UE8XghWAibYE1xuiTx8fyMBk1w2SuRYt2G2cczR95dIlhd64falla_0nfwNi06XIuz15cJ-2JROnOcbhhD5mAwEVz1wVGUHUxsPsOAIXvaiKBTXK5z0m00

```
@startuml
database "Queue" {
}
database "Accounts" {
}
[MSA] -up- Submission
[MTA] -up- Relay
[QM]
[Guard] -left- Check


node "Mailbox system" {
  [Mailbox] -left- Delivery
  [Mailbox] -right- Mail
}

cloud internet {
  [Another Server] - AnotherRelay
}
cloud user {
  [MUA]
}

[MUA] --> Submission: ESMTP
Mail <-- [MUA]: IMAP
[MSA] -down-> Accounts: 1. auth
[MSA] -left-> Queue: 2. store file
[MTA] -down-> Check: 1. LMTP
[MTA] -right-> Queue: 2. store file
[Guard] -> Accounts: rules
[QM] -down-> Queue: pick file
[QM] -left-> AnotherRelay : ESMTP
[QM] -right-> Delivery: LMTP
[Another Server] -down-> Relay: ESMTP
@enduml
```