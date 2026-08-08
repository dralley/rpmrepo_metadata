# Comps: Package Group Metadata in RPM Repositories

Comps (short for "components") is the metadata format RPM-based distributions
use to organize packages into logical groups for installation. The metadata
lives in a file called `comps.xml` (referenced as the `group` record type in
`repomd.xml`) and defines four top-level element types: groups, categories,
environments, and langpacks. Two additional legacy element types (blacklist and
whiteout) exist in the format but are defunct.

This document is cross-referenced against libcomps (the canonical C library),
libsolv (the solver used by DNF), and DNF5's documented behavior.

## File location

The comps data is stored as a compressed XML file in the repository's
`repodata/` directory. Its location is declared in `repomd.xml` under a
`<data type="group">` record:

```xml
<data type="group">
  <location href="repodata/abc123-comps.xml.gz"/>
  <checksum type="sha256">abc123...</checksum>
  ...
</data>
```

The DTD declaration at the top of every comps.xml is:

```xml
<!DOCTYPE comps PUBLIC '-//Red Hat, Inc.//DTD Comps info//EN' 'comps.dtd'>
```

Note: the shipped DTD (in libcomps) is incomplete — it omits several fields
and attributes that libcomps itself parses and that appear in real-world comps
files. This document describes the full format as implemented, not just what
the DTD declares.

## Package Groups

A group is a named collection of packages. Each group has an id, human-readable
name and description (with optional translations), and a list of packages.
Groups are the fundamental building block — environments and categories
reference groups by id, not packages directly.

### XML structure

```xml
<group>
  <id>core</id>
  <name>Core</name>
  <name xml:lang="ja">コア</name>
  <description>Smallest possible installation.</description>
  <description xml:lang="ja">最小限のインストール</description>
  <default>false</default>
  <uservisible>true</uservisible>
  <biarchonly>false</biarchonly>
  <langonly>sr</langonly>
  <display_order>1</display_order>
  <packagelist>
    <packagereq type="mandatory">bash</packagereq>
    <packagereq type="default">vim-minimal</packagereq>
    <packagereq type="optional">zsh</packagereq>
    <packagereq type="conditional" requires="gtk3">ibus-gtk3</packagereq>
    <packagereq type="mandatory" basearchonly="true">grub2-efi-x64</packagereq>
  </packagelist>
</group>
```

### Group fields

| Field | Description |
|-------|-------------|
| `id` | Machine-readable identifier, used to reference the group from environments and categories. |
| `name` | Human-readable name. May have `xml:lang` variants for localization. |
| `description` | Human-readable description. May have `xml:lang` variants. |
| `default` | Whether the group is pre-selected in graphical installers (boolean). Default: `false`. Does not affect `dnf group install`. |
| `uservisible` | Whether the group is shown in UI listings (boolean). Default: `true` (libsolv default). Groups with `<uservisible>false</uservisible>` are hidden unless `--hidden` is passed. |
| `biarchonly` | If `true`, only biarch packages from this group are installed on multilib systems (boolean). |
| `langonly` / `lang_only` | Language code — restricts the group to systems configured for that language. Both element names are accepted (libsolv handles both). |
| `display_order` | Integer controlling sort order in UI listings (lower = earlier). Not in the DTD for groups, but libcomps and libsolv both support it. |
| `packagelist` | List of `<packagereq>` elements (see below). |

The `<group>` element itself may carry an `arch` attribute for architecture
filtering (parsed by libcomps, not in the DTD). This is rarely used in
practice.

### Package requirements

Each `<packagereq>` within a `<packagelist>` has a `type` attribute and
optional additional attributes:

```xml
<packagereq type="mandatory">bash</packagereq>
<packagereq type="default">vim-minimal</packagereq>
<packagereq type="optional">zsh</packagereq>
<packagereq type="conditional" requires="gtk3">ibus-gtk3</packagereq>
<packagereq type="mandatory" basearchonly="true">grub2-efi-x64</packagereq>
```

#### Package types

| Type | Behavior |
|------|----------|
| `mandatory` | Always installed when the group is installed. |
| `default` | Installed by default, but can be excluded. |
| `optional` | Not installed by default. Included with `--with-optional`. |
| `conditional` | Installed only if the package named in the `requires` attribute is also being installed. |

#### Package attributes

| Attribute | Description |
|-----------|-------------|
| `type` | One of `mandatory`, `default`, `optional`, `conditional`. Some comps files in the wild omit this attribute; libcomps defaults to `default` when absent. |
| `requires` | The package name that triggers installation of this conditional package. Only meaningful when `type="conditional"`. |
| `basearchonly` | Boolean. If `true`, the package is only installed for the system's base architecture, suppressing multilib variants. |
| `arch` | Comma/space-separated list of architectures this package applies to. Parsed by libcomps, rarely used. |

### How libsolv represents package types

libsolv maps comps package types to its standard dependency relations:

| Comps type | libsolv relation |
|------------|-----------------|
| `mandatory` | `SOLVABLE_REQUIRES` |
| `default` | `SOLVABLE_RECOMMENDS` |
| `optional` | `SOLVABLE_SUGGESTS` |
| `conditional` | `SOLVABLE_RECOMMENDS` with a `REL_COND` relation linking the package to its `requires` dependency |

### DNF5 default behavior

DNF5's `group_package_types` configuration option controls which package types
are installed by `dnf5 group install`. The default is `mandatory, default,
conditional` — meaning mandatory and default packages are always installed, and
conditional packages are installed when their requirement is met. Optional
packages are excluded unless `--with-optional` is passed.

This is configurable in `dnf5.conf`:

```ini
group_package_types=mandatory, default, conditional
```

## Environments

An environment defines a complete installation profile — a higher-level
grouping of groups. These are the choices presented during OS installation
(e.g., "Minimal Install", "Server", "Workstation"). Each environment has a
mandatory group list and an optional group list.

### XML structure

```xml
<environment>
  <id>server-product-environment</id>
  <name>Server</name>
  <name xml:lang="ja">サーバー</name>
  <description>An integrated, easy-to-manage server.</description>
  <description xml:lang="ja">統合された、管理が容易なサーバーです。</description>
  <display_order>2</display_order>
  <grouplist>
    <groupid>core</groupid>
    <groupid>hardware-support</groupid>
    <groupid>server-product</groupid>
  </grouplist>
  <optionlist>
    <groupid default="true">headless-management</groupid>
    <groupid>debugging</groupid>
    <groupid>network-server</groupid>
  </optionlist>
</environment>
```

### Environment fields

| Field | Description |
|-------|-------------|
| `id` | Machine-readable identifier. |
| `name` | Human-readable name, with optional `xml:lang` variants. |
| `description` | Human-readable description, with optional `xml:lang` variants. |
| `display_order` | Integer controlling sort order in UI listings. |
| `grouplist` | Groups that are always installed with this environment. Contains `<groupid>` elements. |
| `optionlist` | Groups that may optionally be installed. Each `<groupid>` may carry a `default` attribute. |

The `<environment>` element itself may carry an `arch` attribute (libcomps).

### Mandatory vs optional groups

The `<grouplist>` contains groups that are unconditionally installed when the
environment is selected.

The `<optionlist>` contains groups that the user can choose to include. Each
`<groupid>` entry in the optionlist may carry a `default="true"` attribute,
meaning that group is selected by default but can be deselected. Entries
without `default="true"` are not selected by default.

DNF5's `dnf5 environment install` installs all mandatory groups plus
default-flagged optional groups. Non-default optional groups must be selected
explicitly.

Note: `--with-optional` on `dnf5 environment install` controls optional
*packages* within groups, not optional *groups* within the environment.

### How libsolv represents environment groups

libsolv maps environment group membership to dependency relations:

| Group membership | libsolv relation |
|-----------------|-----------------|
| `<grouplist>` (mandatory) | `SOLVABLE_REQUIRES` |
| `<optionlist>` with `default="true"` | `SOLVABLE_RECOMMENDS` |
| `<optionlist>` without default (or `default="false"`) | `SOLVABLE_SUGGESTS` |

Group references are always prefixed with `group:` in libsolv's internal
representation (e.g., `group:core`).

### Relationship to groups

Environments form a two-level hierarchy:

```
Environment (server-product-environment)
  ├── mandatory: core → packages...
  ├── mandatory: hardware-support → packages...
  ├── mandatory: server-product → packages...
  ├── optional (default): headless-management → packages...
  ├── optional: debugging → packages...
  └── optional: network-server → packages...
```

The environment itself contains no packages directly — it only references
groups by id. Each group then expands to its own package list with its own
mandatory/default/optional/conditional categorization.

## Categories

A category groups groups for display purposes in graphical package managers.
Categories organize the group list into sections (e.g., "Development",
"Servers", "System") in tools like Anaconda's package selection screen.

### XML structure

```xml
<category>
  <id>servers</id>
  <name>Servers</name>
  <name xml:lang="ja">サーバー</name>
  <description>Server software and related tools.</description>
  <display_order>30</display_order>
  <grouplist>
    <groupid>file-server</groupid>
    <groupid>mail-server</groupid>
    <groupid>network-server</groupid>
  </grouplist>
</category>
```

### Category fields

| Field | Description |
|-------|-------------|
| `id` | Machine-readable identifier. |
| `name` | Human-readable name, with optional `xml:lang` variants. |
| `description` | Human-readable description, with optional `xml:lang` variants. |
| `display_order` | Integer controlling sort order. |
| `grouplist` | Groups belonging to this category. Contains `<groupid>` elements. |

The `<category>` element itself may carry an `arch` attribute (libcomps).

### Relevance to dependency resolution

Categories have no effect on what gets installed. They are purely
organizational metadata for UI presentation. DNF5 has no `category install`
command. A dependency resolver can safely ignore categories entirely.

libsolv does parse categories (creating `category:ID` solvables with the
groups as `SOLVABLE_REQUIRES`), but this is for query purposes only.

## Langpacks

Langpacks define mappings from package names to their language pack
installation patterns. They tell the package manager how to automatically
install language-specific subpackages when a base package is installed on a
system configured for a particular locale.

### XML structure

```xml
<langpacks>
  <match name="firefox" install="firefox-langpacks-%{lang}"/>
  <match name="libreoffice-core" install="libreoffice-langpack-%{lang}"/>
  <match name="hunspell" install="hunspell-%{lang}"/>
</langpacks>
```

### How it works

Each `<match>` element maps a base package name to an install pattern. The
`%{lang}` placeholder is expanded to the system's configured language code(s).
When a user installs `firefox` on a system configured for French (`fr`), the
langpacks mechanism triggers automatic installation of `firefox-langpacks-fr`.

There is at most one `<langpacks>` element per comps.xml file.

### Relevance to dependency resolution

Langpacks are a post-resolution augmentation. libsolv does not parse langpacks
at all. In DNF4, this was handled by the `dnf-langpacks` plugin. In DNF5, the
langpacks functionality is built in but still operates outside of the core
solver — it is not expressed as RPM dependency relations. A resolver
implementation can safely ignore langpacks.

## Legacy elements: blacklist and whiteout

Two additional top-level elements exist in the comps format but are effectively
dead:

### Blacklist

```xml
<blacklist>
  <package name="some-package" arch="x86_64"/>
</blacklist>
```

Lists packages that should be excluded from installation. Each `<package>`
entry has a `name` attribute and an optional `arch` attribute.

### Whiteout

```xml
<whiteout>
  <ignoredep package="some-package" requires="some-dep"/>
</whiteout>
```

Lists dependency requirements that should be ignored during resolution. Each
`<ignoredep>` entry has a `package` attribute and a `requires` attribute naming
the dependency to suppress.

### Status

libcomps defines the XML elements for both types but their post-processing
callbacks are commented out (`NULL`). libsolv does not parse them. No modern
comps.xml files include them. They are vestigial from an era before RPM's
dependency system was mature enough to handle all cases. They can be safely
ignored.

## Naming conventions

Different tools use different naming conventions for comps solvables:

| Tool | Groups | Environments | Categories |
|------|--------|-------------|------------|
| libsolv | `group:ID` | `environment:ID` | `category:ID` |
| DNF5 CLI | `@ID` or `@Name` | `@ID` or `@Name` | N/A |

DNF5 uses the `@` prefix for both groups and environments in CLI contexts
(e.g., `dnf5 install @core`, `dnf5 install @minimal-environment`). When both a
group and an environment match the same spec, `dnf5 group install` prefers
groups and `dnf5 environment install` prefers environments. Other commands like
`dnf5 install` affect both.

## DNF5 CLI commands

| Command | What it does |
|---------|-------------|
| `dnf5 group list` | List groups (add `--hidden` to include non-uservisible groups). |
| `dnf5 group info <spec>` | Show group details and package list. |
| `dnf5 group install <spec>` | Install a group or environment (prefers groups on ambiguity). Installs mandatory + default + conditional packages. |
| `dnf5 group install --with-optional <spec>` | Also install optional packages within groups. |
| `dnf5 environment list` | List environments. |
| `dnf5 environment info <spec>` | Show environment details and group list. |
| `dnf5 environment install <spec>` | Install an environment (all mandatory groups + default optional groups). Prefers environments on ambiguity. |
| `dnf5 environment install --with-optional <spec>` | Also install optional *packages* within groups (not optional groups). |

## Hierarchy summary

```
comps.xml
├── <group>* ─────────── Contains packages (mandatory/default/optional/conditional)
├── <environment>* ───── References groups via <grouplist> and <optionlist>
├── <category>* ──────── References groups for UI organization only
├── <langpacks>? ─────── Maps package names to language pack patterns
├── <blacklist>? ─────── Legacy, defunct
└── <whiteout>? ──────── Legacy, defunct
```

## Which types matter for a dependency resolver

| Type | Resolver-relevant | Why |
|------|-------------------|-----|
| Groups | Yes | Expand to concrete package requirements. |
| Environments | Yes | Expand to groups, which expand to packages. |
| Categories | No | UI-only organization, no install semantics. |
| Langpacks | No | Post-resolution concern, not dependency metadata. |
| Blacklist | No | Legacy, defunct. |
| Whiteout | No | Legacy, defunct. |

## Architecture filtering

libcomps supports an `arch` attribute on `<group>`, `<category>`,
`<environment>`, `<packagereq>`, and `<groupid>` elements. This is a
comma/space-separated list of architectures the element applies to. When
present, the element should only be processed if the target architecture
matches. libsolv does not process these attributes — architecture filtering
is expected to happen at a higher level.

## Comps across repositories

A single comps.xml file may appear in multiple repositories (e.g., BaseOS and
AppStream each carry their own comps.xml). The same group id can appear in
multiple repos' comps files — the sets of groups, environments, and categories
across repos are merged. When duplicates exist, the convention is that the
last-loaded definition wins, though in practice groups with the same id across
repos typically have identical content.

## Comps in Fedora vs RHEL

Both Fedora and RHEL/CentOS use the same comps.xml format. The content differs
(different groups, environments, and package lists), but the schema is
identical. The format originates from Red Hat's Anaconda installer and has been
stable for over 15 years.

The comps files for Fedora are maintained in
[fedora-comps](https://pagure.io/fedora-comps). RHEL comps files are internal
to Red Hat but follow the same format.
