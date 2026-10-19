# SolidCraft Holz — native Möbelbau-Erweiterung

Dieser SolidCraft-Fork enthält das Möbelbaumodul direkt in derselben Anwendung. Es startet kein Zusatzprogramm und benötigt keinen externen Generator. Einstieg: **Werkzeuge → Erweiterungen → Möbelkorpus** oder Befehlssuche **Möbelkorpus**. **Möbel-Stückliste** öffnet denselben Dialog.

## Ablauf

1. Optional einen achsenparallelen Quader im Modellbaum auswählen und im Möbelbaudialog **Maße vom ausgewählten Quader übernehmen** anklicken. Seine Position und Außenmaße werden einmalig übernommen. Der Grundkörper wird nicht gelöscht; **Referenz ausblenden** verbirgt ihn bei Bedarf.
2. Plattenstärke, Böden je Fach, senkrechte Trennwände, Rückwand und Türen einstellen. Material, Dichte, Kantenband und Rohmaßzugaben eingeben.
3. **Korpus im Dokument erstellen** fügt die Platten zum aktuellen Dokument hinzu. Vorhandene Konstruktionen bleiben erhalten. Mehrere Korpusse erhalten unabhängige Parameter (`Holz1_Breite`, `Holz2_Breite` usw.).
4. Ein Rückgängig-Schritt entfernt den gesamten erzeugten Korpus. Wiederholen stellt ihn wieder her. Die erzeugten Platten sind normale native parametrische Formelemente; das Dokument kann auch ohne dieses Modul geöffnet werden.
5. **Stückliste aktualisieren** oder **Aktuelle Stückliste als CSV speichern** misst die aktuellen Körper. Änderungen der Korpusparameter werden damit in die Maße der Liste übernommen. Material, Dichte und Bekantung stammen aus der Korpuserstellung. Den CSV-Pfad im Dialog festlegen.

Die Liste bezieht sich auf den zuletzt erstellten Korpus. Programmatisch kann `woodwork.bom` einen anderen Korpus über `instance` auswählen. Fehlende oder umbenannte Platten werden als Fehler gemeldet. Geänderte Konturteile werden in der CSV zur Konturprüfung gekennzeichnet.

## Native Befehle

- `woodwork.cabinet`: `config` mit Breite, Höhe, Tiefe, Plattenstärke, Böden, Material und `dividers` (0–4), optional `origin` in mm. Transaktion mit einem Rückgängig-Schritt.
- `woodwork.bounds`: `body` — Außenmaße und Ursprung eines achsenparallelen Quaders, ohne Geometrieänderung.
- `woodwork.bom`: optional `instance` und `path` — aktuelle Plattenmaße und CSV, optional Dateiexport.

## Konstruktionsumfang

Rechteckige Korpusse, durchgehende Seiten, Boden/Deckel dazwischen, bis zu vier senkrechte Trennwände und zwanzig Bodenreihen mit getrennten Platten je Fach. Rückwand innerhalb der Korpustiefe, optionale aufliegende Doppeltür. Stumpfe Plattenstöße ohne geometrische Überlappungen. Kantenband und Maserung sind Fertigungsangaben, keine separate Geometrie bzw. Textur.

Das [Referenzvideo](https://www.youtube.com/watch?v=m3m55fZEWLU) wurde anhand seines Bildinhalts und des verfügbaren deutschen Transkripts ausgewertet. Eigenständige Implementierung; keine fremden Programmdateien, Grafiken oder Vorlagen übernommen. Schräge Grundkörper, Gehrungen und Trimmungen sind in der unten beschriebenen Ergänzung enthalten. Die Maße der Referenzgeometrie bleiben nach ihrer Übernahme nicht mit ihr verknüpft.

Dies ist ein fest eingebautes Modul in diesem Fork. SolidCraft stellt in diesem Quellstand noch keinen fertigen Lader für unabhängig installierbare Plug-ins bereit; `docs/plugins.md` beschreibt eine geplante Schnittstelle.

## Ergänzung: Konturplatten, Gehrung und freie Trimmung

Unter **Werkzeuge → Erweiterungen** stehen außerdem **Grundkörper in Platten**, **Möbel-Gehrung** und **Möbelplatte trimmen** zur Verfügung. Sie öffnen die erweiterten Werkzeuge im selben nativen Möbelbaudialog.

### Schräge Grundkörper

- Einen konvexen Volumenkörper mit 4–12 ebenen Außenflächen wählen. Dazu gehören Quader, gedrehte Quader, Keile und schräge prismatische Formen.
- Öffnungsfläche über die Flächenliste oder durch Auswahl einer Modellfläche übernehmen. Die Öffnung bleibt frei.
- Eine Plattenstärke angeben und **Grundkörper in Gehrungsplatten aufteilen** anklicken.
- Für jede andere Außenfläche wird eine Platte erzeugt. Ihre Nachbargrenzen ergeben sich aus den Winkelhalbierenden der jeweiligen Außenebenen; dadurch entstehen auch bei schrägen Korpussen passende Gehrungen. Die ursprüngliche Referenz bleibt erhalten und kann mit dem Dialogknopf ausgeblendet werden.
- Keine gekrümmten oder konkaven Grundkörper, Aussparungen im Referenzkörper oder beliebige Freiformflächen. Sehr dünne und degenerierte Geometrie wird abgewiesen. Hiermit entstehen die Außenplatten; das Einpassen von Einlegeböden in einen schrägen Konturkorpus ist noch kein automatischer Arbeitsablauf.

### Gehrung zwischen vorhandenen Platten

Zwei überlappende Platten und jeweils eine äußere Breitfläche wählen. Die Flächen können direkt im Modell mehrfach ausgewählt und dann übernommen werden. **Gehrung schneiden** kürzt beide Platten an der Winkelhalbierenden. Unpassende oder parallele Flächen werden zurückgewiesen. Ein Rückgängig-Schritt stellt beide Körper wieder her.

### Freie Trimmung

Eine Platte auswählen. Entweder eine Schnittebene durch Punkt und Normale definieren beziehungsweise eine Modellfläche als Ebene übernehmen und die zu behaltende Seite bestimmen, oder einen Grenzkörper abziehen. Der Grenzkörper bleibt erhalten. Ein Ebenenschnitt muss genau ein zusammenhängendes Restteil behalten; das vollständige Entfernen wird abgewiesen. Körperschnitte nutzen die vorhandenen nativen Booleschen Operationen.

### Zuschnittliste und Geometrie

Konturplatten erhalten projizierte Hüllmaße in ihrer Plattenebene. Volumen und Masse kommen aus der tatsächlichen Geometrie, nicht aus dem umschließenden Quader. Die CSV kennzeichnet solche Teile mit **Konturprüfung**; für Gehrungen, Schrägen und Aussparungen reicht eine rechteckige Zuschnittmaßangabe allein nicht aus. Kantenband wird für automatisch erzeugte Konturplatten nicht pauschal zugewiesen.

Alle Bearbeitungen werden als native Formelemente in der Historie gespeichert. Die ermittelten Schnittebenen werden bei der Erstellung festgelegt; ein später veränderter Referenzkörper berechnet die Winkelhalbierenden nicht automatisch neu. Für eine neue Referenzform die Aufteilung neu erzeugen.

## Aus dem Quellcode bauen

```sh
cargo build --release -p solidcraft -p solidcraft-cli
./target/release/solidcraft
```

Die normalen Downloadlinks im Haupt-README führen zu offiziellen SolidCraft-Versionen. Diese enthalten die Möbelbau-Erweiterung erst nach einer eigenen Übernahme; zum Testen dieses Forks den Branch mit der Erweiterung bauen. Maßfelder mit Enter oder durch Verlassen des Feldes bestätigen; beispielsweise lässt sich 19 mm ohne vorzeitiges Begrenzen auf 3 mm eingeben.
