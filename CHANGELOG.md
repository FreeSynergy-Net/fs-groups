# Changelog

## Unreleased

### English
- Core only (the window follows later; no trait, wire or window change): fixed, typed role list `Role` (member, manager, owner; texts from before the list read as `Role::Legacy`) with `add_role` and `set_role`.
- The last owner (with no owner: the last manager) cannot be removed, demoted or leave (`Error::LastOwner`, `Error::LastManager`); one rule in `protect::check_change`.
- `transfer_ownership` hands the ownership over in one step; `delete_by` lets only an owner delete a group; `leave` ends a person's membership.
- Nesting mode per edge (ADR 0082): `NestingMode` BottomUp / TopDown / Single, preselected per parent (`set_default_mode`), asked with `request_parent_in`, changed with `request_mode_change` (consent) or `set_single` (at once). Edges without a recorded mode stay BottomUp.
- `effective_origins` answers per person: direct, or inherited from which group along which path, modes respected (`effective`, `groups_of`, `is_member` follow the modes too).

### Deutsch
- Nur Kern (das Fenster folgt später; keine Änderung an Trait, Wire oder Fenster): feste, typisierte Rollenliste `Role` (Mitglied, Verwalter, Eigentümer; Texte von vor der Liste lesen sich als `Role::Legacy`) mit `add_role` und `set_role`.
- Der letzte Eigentümer (ohne Eigentümer: der letzte Verwalter) kann nicht entfernt, herabgestuft werden und nicht austreten (`Error::LastOwner`, `Error::LastManager`); eine Regel in `protect::check_change`.
- `transfer_ownership` überträgt das Eigentum in einem Schritt; `delete_by` lässt nur einen Eigentümer die Gruppe löschen; `leave` beendet die Mitgliedschaft einer Person.
- Vererbungsmodus je Kante (ADR 0082): `NestingMode` BottomUp / TopDown / Einzeln, je Eltern vorgewählt (`set_default_mode`), angefragt mit `request_parent_in`, geändert mit `request_mode_change` (Zustimmung) oder `set_single` (sofort). Kanten ohne gespeicherten Modus bleiben BottomUp.
- `effective_origins` antwortet je Person: direkt oder geerbt von welcher Gruppe über welchen Pfad, Modi beachtet (`effective`, `groups_of`, `is_member` folgen den Modi ebenfalls).

## 0.0.2

### English
- `fs-groups --version`/`-V` prints the version and exits without opening a window or a server (for the release smoke test).
- The `fs-groups` binary is the one program: the window (feature `gui`, on by default) lists, reads, creates, renames, sets the kind of and deletes groups over the Cap'n Proto socket; `fs-groups serve` runs the server.
- Package metadata (`authors`, `repository`) is declared in all crates, as the release flow requires.
- Earlier versions: see the Git tags.

### Deutsch
- `fs-groups --version`/`-V` gibt die Version aus und endet, ohne ein Fenster oder einen Server zu öffnen (für den Release-Smoke-Test).
- Die Binärdatei `fs-groups` ist das eine Programm: Das Fenster (Feature `gui`, standardmäßig an) listet, liest, legt an, benennt um, setzt die Art und löscht Gruppen über den Cap'n-Proto-Socket; `fs-groups serve` startet den Server.
- Paket-Metadaten (`authors`, `repository`) sind in allen Crates angegeben, wie der Release-Ablauf es verlangt.
- Frühere Versionen: siehe Git-Tags.
