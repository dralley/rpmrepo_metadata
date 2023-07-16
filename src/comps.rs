<?xml encoding="UTF-8"?>

<!ELEMENT comps (group+,environment+,category+,langpacks?)>
<!ATTLIST comps xmlns CDATA #FIXED ''>

<!ELEMENT group (id,name,description,default,uservisible,langonly?, packagelist)>
<!ATTLIST group xmlns CDATA #FIXED ''>

<!ELEMENT environment (id,name,description,display_order?,grouplist,optionlist)>
<!ATTLIST environment xmlns CDATA #FIXED ''>

<!ELEMENT category (id,name,description,display_order?,grouplist)>
<!ATTLIST category xmlns CDATA #FIXED ''>

<!ELEMENT default (#PCDATA)>
<!ATTLIST default xmlns CDATA #FIXED ''>

<!ELEMENT uservisible (#PCDATA)>
<!ATTLIST uservisible xmlns CDATA #FIXED ''>

<!ELEMENT langonly (#PCDATA)>
<!ATTLIST langonly xmlns CDATA #FIXED ''>

<!ELEMENT packagelist (packagereq)+>
<!ATTLIST packagelist xmlns CDATA #FIXED ''>

<!ELEMENT display_order (#PCDATA)>
<!ATTLIST display_order xmlns CDATA #FIXED ''>

<!ELEMENT grouplist (groupid)+>
<!ATTLIST grouplist xmlns CDATA #FIXED ''>

<!ELEMENT packagereq (#PCDATA)>
<!ATTLIST packagereq xmlns CDATA #FIXED '' requires NMTOKEN #IMPLIED type NMTOKEN #REQUIRED>

<!ELEMENT groupid (#PCDATA)>
<!ATTLIST groupid xmlns CDATA #FIXED ''>

<!ELEMENT id (#PCDATA)>
<!ATTLIST id xmlns CDATA #FIXED ''>

<!ELEMENT name (#PCDATA)>
<!ATTLIST name xmlns CDATA #FIXED ''>

<!ELEMENT description (#PCDATA)>
<!ATTLIST description xmlns CDATA #FIXED ''>

<!ELEMENT optionlist (groupid)+>
<!ATTLIST optionlist xmlns CDATA #FIXED ''>




<?xml version='1.0' encoding='UTF-8'?>
<!DOCTYPE comps PUBLIC "-//Red Hat, Inc.//DTD Comps info//EN" "comps.dtd">
<comps>
  <group>
    <id>additional-devel</id>
    <name>Additional Development</name>
    <description>Additional development headers and libraries for developing applications</description>
    <default>false</default>
    <uservisible>false</uservisible>
    <biarchonly>true</biarchonly>
    <langonly>fr</langonly>
    <packagelist>
      <packagereq type="default">alsa-lib-devel</packagereq>
      <packagereq type="default">audit-libs-devel</packagereq>
      <packagereq type="default">binutils-devel</packagereq>
      <packagereq type="default">boost-devel</packagereq>
      <packagereq type="default">bzip2-devel</packagereq>
      <packagereq type="default">cyrus-sasl-devel</packagereq>
    </packagelist>
  </group>
  <group>
    <id>backup-client</id>
    <name>Backup Client</name>
    <description>Client tools for connecting to a backup server and doing backups.</description>
    <default>true</default>
    <uservisible>true</uservisible>
    <packagelist>
      <packagereq type="mandatory">amanda-client</packagereq>
      <packagereq type="optional">bacula-client</packagereq>
    </packagelist>
  </group>
  <group>
    <id>backup-server</id>
    <name>Backup Server</name>
    <description>Software to centralize your infrastructure's backups.</description>
    <default>false</default>
    <uservisible>true</uservisible>
    <packagelist>
      <packagereq type="mandatory">amanda-server</packagereq>
      <packagereq type="optional">mt-st</packagereq>
      <packagereq type="optional">mtx</packagereq>
    </packagelist>
  </group>
  <group>
    <id>ansible-node</id>
    <name>Ansible node</name>
    <default>false</default>
    <uservisible>true</uservisible>
    <packagelist>
      <packagereq type="mandatory">python2-dnf</packagereq>
      <packagereq type="conditional" requires="selinux-policy">libselinux-python</packagereq>
    </packagelist>
  </group>
  <group>
    <id>d-development</id>
    <name>D Development Tools and Libraries</name>
    <description>These include development tools and libraries such as ldc, and geany-tag.</description>
    <default>false</default>
    <uservisible>true</uservisible>
    <packagelist>
      <packagereq type="mandatory" basearchonly="true">ldc</packagereq>
      <packagereq type="mandatory" basearchonly="true">ldc-druntime</packagereq>
      <packagereq type="mandatory" basearchonly="true">ldc-druntime-devel</packagereq>
      <packagereq type="mandatory" basearchonly="true">ldc-phobos-devel</packagereq>
      <packagereq type="mandatory">make</packagereq>
      <packagereq type="mandatory">pkgconfig</packagereq>
      <packagereq type="default">ctags</packagereq>
      <packagereq type="default">indent</packagereq>
      <packagereq type="optional">astyle</packagereq>
      <packagereq type="optional">cmake</packagereq>
      <packagereq type="optional">derelict-devel</packagereq>
      <packagereq type="optional">geany</packagereq>
      <packagereq type="optional">gl3n-devel</packagereq>
      <packagereq type="optional">insight</packagereq>
      <packagereq type="optional">nemiver</packagereq>
      <packagereq type="optional">uncrustify</packagereq>
    </packagelist>
  </group>
  <group>
    <id>empty-group-1</id>
    <name>empty group 1</name>
    <description>empty group 1 desc</description>
    <default>false</default>
    <uservisible>true</uservisible>
    <packagelist/>
  </group>
  <group>
    <id>empty-group-2</id>
    <name>empty group 2</name>
    <description>empty group 2 desc</description>
    <default>false</default>
    <uservisible>true</uservisible>
  </group>
    <group>
    <id>unknown-group</id>
    <name>unknown group</name>
    <description>unknown group desc</description>
    <default>false</default>
    <uservisible>true</uservisible>
    <packagelist>
      <packagereq type="unknown">unknown</packagereq>
      <packagereq type="what">unknown2</packagereq>
    </packagelist>
  </group>
</comps>
