import QtQuick

// Symbolic 16px icons for Hue room and light archetypes, drawn in any color
// (Kirigami.Icon only recolors icons that come from the icon theme).
QtObject {
    readonly property string base: "qrc:/qt/qml/io/github/pcmciakai/kandela/qml/icons/"

    readonly property var paths: ({
        bulb: "M8 1a5 5 0 0 0-3 9c.6.5 1 1.1 1 1.8V12h4v-.2c0-.7.4-1.3 1-1.8a5 5 0 0 0-3-9zm0 1.5A3.5 3.5 0 0 1 10.1 8.8c-.8.6-1.4 1.4-1.6 2.2h-1c-.2-.8-.8-1.6-1.6-2.2A3.5 3.5 0 0 1 8 2.5zM6 13v1h4v-1zm1 1.5V15h2v-.5z",
        lamp: "M5 1 3 7h4.25v6H5v1.5h6V13H8.75V7H13l-2-6zm1.1 1.5h3.8l1 3H5.1z",
        strip: "M2 5a2 2 0 0 0-2 2v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2zm0 1.5h12a.5.5 0 0 1 .5.5v2a.5.5 0 0 1-.5.5H2a.5.5 0 0 1-.5-.5V7a.5.5 0 0 1 .5-.5zM3 7.25v1.5h1.5v-1.5zm3 0v1.5h1.5v-1.5zm3 0v1.5h1.5v-1.5zm3 0v1.5h1v-1.5z",
        sofa: "M4 3a2 2 0 0 0-2 2v1.05A2 2 0 0 0 0 8v4h1.5v1H3v-1h10v1h1.5v-1H16V8a2 2 0 0 0-2-1.95V5a2 2 0 0 0-2-2zm0 1.5h8a.5.5 0 0 1 .5.5v1.3A2 2 0 0 0 12 8v1H4V8a2 2 0 0 0-.5-1.3V5a.5.5 0 0 1 .5-.5zM2 7.5a.5.5 0 0 1 .5.5v2.5h11V8a.5.5 0 0 1 1 0v2.5h0V10.5H1.5V8a.5.5 0 0 1 .5-.5z",
        bed: "M1 3v11h1.5v-2h11v2H15V8.5A2.5 2.5 0 0 0 12.5 6H7v3.5H2.5V3zm7.5 4.5h4a1 1 0 0 1 1 1v2h-5zM4.5 6a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM2.5 10.5h0z",
        desk: "M3 2v6h4.25v1.5H5V11h6V9.5H8.75V8H13V2zm1.5 1.5h7v3h-7zM1 12v1.5h1.5V15H4v-1.5h8V15h1.5v-1.5H15V12z",
        tree: "M8 1 3 7.5h2.2L2.5 11.5h4.75V15h1.5v-3.5h4.75L10.8 7.5H13zm0 2.4 2 2.6H8.1l2.6 4H5.3l2.6-4H6z",
        kitchen: "M5 1v2h1.5V1zm4.5 0v2H11V1zM2 5v1.5h1V12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V6.5h1V5zm2.5 1.5h7V12a.5.5 0 0 1-.5.5H5a.5.5 0 0 1-.5-.5z",
        house: "M8 1 1 7.2l1 1.1 1-.9V15h4v-4h2v4h4V7.4l1 .9 1-1.1zm0 2 3.5 3.1v7.4h-1v-4h-5v4h-1V6.1z",
    })

    function svg(name, color) {
        const d = paths[name] ?? paths.bulb;
        const c = Qt.color(color);
        const fill = `rgb(${Math.round(c.r * 255)},${Math.round(c.g * 255)},${Math.round(c.b * 255)})`;
        return "data:image/svg+xml;utf8," + encodeURIComponent(
            `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path fill="${fill}" fill-opacity="${c.a}" d="${d}"/></svg>`);
    }

    function room(archetype, color) {
        const groups = {
            sofa: ["living_room", "lounge", "tv", "recreation", "man_cave", "music"],
            bed: ["bedroom", "kids_bedroom", "guest_room", "nursery"],
            desk: ["office", "computer", "studio", "reading"],
            tree: ["garden", "terrace", "balcony", "backyard", "driveway", "porch", "front_door", "carport", "pool"],
            kitchen: ["kitchen", "dining", "barbecue"],
        };
        for (const icon in groups) {
            if (groups[icon].includes(archetype)) return svg(icon, color);
        }
        return svg("house", color);
    }

    function light(archetype, color) {
        if (/strip|play|gradient|signe/.test(archetype)) return svg("strip", color);
        if (/^(table|floor|desk)|lamp|lantern|shade|wash/.test(archetype)) return svg("lamp", color);
        return svg("bulb", color);
    }
}
