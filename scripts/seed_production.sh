#!/usr/bin/env bash
# ════════════════════════════════════════════════════════════
# VARCAVIA — Seed Production Node con fatti verificabili
# ════════════════════════════════════════════════════════════
#
# Uso: bash scripts/seed_production.sh [URL]
# Default: https://varcavia.com
#
set -euo pipefail

BASE_URL="${1:-https://varcavia.com}"
API="$BASE_URL/api/v1/data"

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

INSERTED=0
DUPLICATES=0
ERRORS=0
TOTAL=0

insert_fact() {
    local content="$1"
    local domain="$2"
    local source="$3"
    TOTAL=$((TOTAL + 1))

    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$API" \
        -H "Content-Type: application/json" \
        -d "{\"content\":\"$content\",\"domain\":\"$domain\",\"source\":\"$source\"}" \
        --max-time 15 2>/dev/null || echo "000")

    if [ "$status" = "201" ]; then
        INSERTED=$((INSERTED + 1))
    elif [ "$status" = "409" ]; then
        DUPLICATES=$((DUPLICATES + 1))
    else
        ERRORS=$((ERRORS + 1))
    fi
}

echo -e "${GREEN}VARCAVIA${NC} — Seed Production Node"
echo "  Target: $BASE_URL"
echo ""

# Verifica raggiungibilita
echo -n "  Checking node... "
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/v1/node/status" --max-time 10 2>/dev/null || echo "000")
if [ "$HTTP_STATUS" = "200" ]; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED (HTTP $HTTP_STATUS)${NC}"
    echo "  Nodo non raggiungibile. Verifica l'URL."
    exit 1
fi
echo ""

# ═══════════════════════════════════════
# EARTH — science
# ═══════════════════════════════════════
insert_fact "Earth is the third planet from the Sun and the only astronomical object known to harbor life." "science" "wikipedia:Earth"
insert_fact "Earth has a mean radius of 6371 kilometres." "science" "wikipedia:Earth"
insert_fact "Earth orbits the Sun at an average distance of 149.6 million km." "science" "wikipedia:Earth"
insert_fact "Earth rotates on its axis once every 23 hours 56 minutes and 4 seconds." "science" "wikipedia:Earth"
insert_fact "The Earth is approximately 4.54 billion years old." "science" "wikipedia:Earth"
insert_fact "Earth has one natural satellite, the Moon." "science" "wikipedia:Earth"
insert_fact "About 71 percent of Earth's surface is covered with water." "science" "wikipedia:Earth"
insert_fact "Earth's atmosphere is composed of 78 percent nitrogen and 21 percent oxygen." "science" "wikipedia:Earth"
echo -e "  [Earth]           ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# FRANCE — geography
# ═══════════════════════════════════════
insert_fact "France is a country in Western Europe with an area of 643,801 square kilometres." "geography" "wikipedia:France"
insert_fact "France has a population of approximately 68 million people." "geography" "wikipedia:France"
insert_fact "Paris is the capital and largest city of France." "geography" "wikipedia:France"
insert_fact "The French Republic was founded in 1792 after the French Revolution." "geography" "wikipedia:France"
insert_fact "France is the largest country in the European Union by area." "geography" "wikipedia:France"
insert_fact "Mont Blanc at 4,808 metres is the highest peak in France." "geography" "wikipedia:France"
insert_fact "France has 13 administrative regions in metropolitan France." "geography" "wikipedia:France"
echo -e "  [France]          ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# UNITED NATIONS — politics
# ═══════════════════════════════════════
insert_fact "The United Nations is an intergovernmental organization founded on 24 October 1945." "politics" "wikipedia:United_Nations"
insert_fact "The United Nations has 193 member states as of 2024." "politics" "wikipedia:United_Nations"
insert_fact "The UN headquarters is located in New York City on 18 acres of land." "politics" "wikipedia:United_Nations"
insert_fact "The United Nations has six principal organs including the General Assembly and Security Council." "politics" "wikipedia:United_Nations"
insert_fact "The UN Security Council has five permanent members with veto power." "politics" "wikipedia:United_Nations"
insert_fact "The United Nations was founded to maintain international peace and security." "politics" "wikipedia:United_Nations"
echo -e "  [United Nations]  ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# SPEED OF LIGHT — science
# ═══════════════════════════════════════
insert_fact "The speed of light in vacuum is exactly 299,792,458 metres per second." "science" "wikipedia:Speed_of_light"
insert_fact "Light travels approximately 9.461 trillion kilometres in one year." "science" "wikipedia:Speed_of_light"
insert_fact "The speed of light is the universal speed limit for all massless particles." "science" "wikipedia:Speed_of_light"
insert_fact "Albert Einstein established that nothing can travel faster than the speed of light." "science" "wikipedia:Speed_of_light"
insert_fact "Light takes approximately 8 minutes and 20 seconds to travel from the Sun to Earth." "science" "wikipedia:Speed_of_light"
insert_fact "The speed of light was first measured by Ole Roemer in 1676." "science" "wikipedia:Speed_of_light"
echo -e "  [Speed of Light]  ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# WATER — science
# ═══════════════════════════════════════
insert_fact "Water is a chemical compound with the formula H2O consisting of two hydrogen atoms and one oxygen atom." "science" "wikipedia:Water"
insert_fact "Water boils at 100 degrees Celsius at standard atmospheric pressure." "science" "wikipedia:Water"
insert_fact "Water freezes at 0 degrees Celsius under standard conditions." "science" "wikipedia:Water"
insert_fact "Water covers approximately 71 percent of the Earth's surface." "science" "wikipedia:Water"
insert_fact "Pure water has a neutral pH of 7." "science" "wikipedia:Water"
insert_fact "Water has a density of approximately 1000 kg per cubic metre at 4 degrees Celsius." "science" "wikipedia:Water"
insert_fact "The human body is composed of approximately 60 percent water." "science" "wikipedia:Water"
echo -e "  [Water]           ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# ALBERT EINSTEIN — science
# ═══════════════════════════════════════
insert_fact "Albert Einstein was a German-born theoretical physicist who lived from 1879 to 1955." "science" "wikipedia:Albert_Einstein"
insert_fact "Einstein developed the theory of relativity, one of the two pillars of modern physics." "science" "wikipedia:Albert_Einstein"
insert_fact "Einstein received the Nobel Prize in Physics in 1921 for his explanation of the photoelectric effect." "science" "wikipedia:Albert_Einstein"
insert_fact "Einstein published his special theory of relativity in 1905." "science" "wikipedia:Albert_Einstein"
insert_fact "Einstein's mass-energy equivalence formula E=mc2 is the world's most famous equation." "science" "wikipedia:Albert_Einstein"
insert_fact "Einstein became a Swiss citizen in 1901 and later an American citizen in 1940." "science" "wikipedia:Albert_Einstein"
echo -e "  [Einstein]        ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# MOON — science
# ═══════════════════════════════════════
insert_fact "The Moon is Earth's only natural satellite at a mean distance of 384,400 km." "science" "wikipedia:Moon"
insert_fact "The Moon has a diameter of 3,474 kilometres, about one-quarter of Earth's diameter." "science" "wikipedia:Moon"
insert_fact "The Moon orbits Earth once every 27.3 days." "science" "wikipedia:Moon"
insert_fact "The Moon's surface gravity is about one-sixth of Earth's gravity." "science" "wikipedia:Moon"
insert_fact "Neil Armstrong became the first person to walk on the Moon on July 20, 1969." "science" "wikipedia:Moon"
insert_fact "The Moon has no atmosphere and no liquid water on its surface." "science" "wikipedia:Moon"
echo -e "  [Moon]            ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# SUN — science
# ═══════════════════════════════════════
insert_fact "The Sun is a G-type main-sequence star comprising 99.86 percent of the Solar System's mass." "science" "wikipedia:Sun"
insert_fact "The Sun has a surface temperature of approximately 5,778 Kelvin." "science" "wikipedia:Sun"
insert_fact "The Sun is approximately 4.6 billion years old." "science" "wikipedia:Sun"
insert_fact "The Sun's diameter is about 1.39 million kilometres, 109 times that of Earth." "science" "wikipedia:Sun"
insert_fact "The Sun converts approximately 600 million tonnes of hydrogen into helium every second." "science" "wikipedia:Sun"
insert_fact "Light from the Sun takes about 8 minutes and 20 seconds to reach Earth." "science" "wikipedia:Sun"
echo -e "  [Sun]             ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# HUMAN BODY — health
# ═══════════════════════════════════════
insert_fact "The adult human body contains approximately 206 bones." "health" "wikipedia:Human_body"
insert_fact "The human heart beats approximately 100,000 times per day." "health" "wikipedia:Human_body"
insert_fact "The human brain contains approximately 86 billion neurons." "health" "wikipedia:Human_body"
insert_fact "The average human body temperature is 37 degrees Celsius." "health" "wikipedia:Human_body"
insert_fact "An adult human body contains about 5 litres of blood." "health" "wikipedia:Human_body"
echo -e "  [Human Body]      ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# DNA — science
# ═══════════════════════════════════════
insert_fact "DNA is a molecule composed of two polynucleotide chains that coil around each other." "science" "wikipedia:DNA"
insert_fact "The human genome contains approximately 3 billion base pairs of DNA." "science" "wikipedia:DNA"
insert_fact "DNA was first identified by Friedrich Miescher in 1869." "science" "wikipedia:DNA"
insert_fact "James Watson and Francis Crick discovered the double helix structure of DNA in 1953." "science" "wikipedia:DNA"
insert_fact "Human DNA is approximately 99.9 percent identical between individuals." "science" "wikipedia:DNA"
insert_fact "DNA contains four nucleotide bases: adenine, thymine, guanine, and cytosine." "science" "wikipedia:DNA"
echo -e "  [DNA]             ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# OXYGEN — science
# ═══════════════════════════════════════
insert_fact "Oxygen is a chemical element with atomic number 8 and symbol O." "science" "wikipedia:Oxygen"
insert_fact "Oxygen makes up approximately 21 percent of Earth's atmosphere by volume." "science" "wikipedia:Oxygen"
insert_fact "Oxygen is the third most abundant element in the universe by mass." "science" "wikipedia:Oxygen"
insert_fact "Oxygen was independently discovered by Carl Wilhelm Scheele and Joseph Priestley in the 1770s." "science" "wikipedia:Oxygen"
insert_fact "Liquid oxygen has a boiling point of minus 183 degrees Celsius." "science" "wikipedia:Oxygen"
echo -e "  [Oxygen]          ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# GOLD — science
# ═══════════════════════════════════════
insert_fact "Gold is a chemical element with atomic number 79 and symbol Au." "science" "wikipedia:Gold"
insert_fact "Gold has a density of 19.3 grams per cubic centimetre." "science" "wikipedia:Gold"
insert_fact "Gold has a melting point of 1,064 degrees Celsius." "science" "wikipedia:Gold"
insert_fact "Approximately 190,000 tonnes of gold have been mined throughout history." "science" "wikipedia:Gold"
insert_fact "Gold is one of the least reactive chemical elements and is resistant to corrosion." "science" "wikipedia:Gold"
echo -e "  [Gold]            ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# PYTHON — technology
# ═══════════════════════════════════════
insert_fact "Python is a high-level programming language first released in 1991." "technology" "wikipedia:Python"
insert_fact "Python was created by Guido van Rossum and named after Monty Python." "technology" "wikipedia:Python"
insert_fact "Python is one of the most popular programming languages in the world." "technology" "wikipedia:Python"
insert_fact "Python supports multiple programming paradigms including object-oriented and functional programming." "technology" "wikipedia:Python"
insert_fact "Python has a comprehensive standard library of over 200 modules." "technology" "wikipedia:Python"
echo -e "  [Python]          ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# INTERNET — technology
# ═══════════════════════════════════════
insert_fact "The Internet is a global system of interconnected computer networks." "technology" "wikipedia:Internet"
insert_fact "The Internet originated from ARPANET, which was established in 1969." "technology" "wikipedia:Internet"
insert_fact "As of 2024, approximately 5.4 billion people worldwide use the Internet." "technology" "wikipedia:Internet"
insert_fact "The World Wide Web was invented by Tim Berners-Lee in 1989." "technology" "wikipedia:Internet"
insert_fact "The Internet uses the TCP/IP protocol suite for data transmission." "technology" "wikipedia:Internet"
echo -e "  [Internet]        ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# TOKYO — geography
# ═══════════════════════════════════════
insert_fact "Tokyo is the capital and largest city of Japan with a population of over 13 million." "geography" "wikipedia:Tokyo"
insert_fact "The Greater Tokyo Area is the most populous metropolitan area in the world with 37 million people." "geography" "wikipedia:Tokyo"
insert_fact "Tokyo was originally known as Edo before being renamed in 1868." "geography" "wikipedia:Tokyo"
insert_fact "Tokyo hosted the Summer Olympic Games in 1964 and 2021." "geography" "wikipedia:Tokyo"
insert_fact "Tokyo's GDP is approximately 1.9 trillion US dollars, making it the wealthiest city globally." "geography" "wikipedia:Tokyo"
echo -e "  [Tokyo]           ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# EXTRA FACTS — mixed domains
# ═══════════════════════════════════════
insert_fact "The speed of sound in air at 20 degrees Celsius is approximately 343 metres per second." "science" "varcavia:hardcoded"
insert_fact "The Great Wall of China stretches over 21,196 kilometres." "geography" "varcavia:hardcoded"
insert_fact "The Amazon River is approximately 6,400 kilometres long." "geography" "varcavia:hardcoded"
insert_fact "Mars has two moons named Phobos and Deimos." "science" "varcavia:hardcoded"
insert_fact "The Pacific Ocean is the largest and deepest ocean covering 165.25 million square kilometres." "geography" "varcavia:hardcoded"
insert_fact "Jupiter is the largest planet in the Solar System with a diameter of 142,984 km." "science" "varcavia:hardcoded"
insert_fact "The Mariana Trench is the deepest known part of the ocean at 10,994 metres." "geography" "varcavia:hardcoded"
insert_fact "The human genome was first fully sequenced in 2003." "science" "varcavia:hardcoded"
insert_fact "The International Space Station orbits Earth at approximately 408 kilometres altitude." "science" "varcavia:hardcoded"
insert_fact "Mount Everest is the highest mountain on Earth at 8,849 metres above sea level." "geography" "varcavia:hardcoded"
insert_fact "The Sahara Desert covers approximately 9.2 million square kilometres." "geography" "varcavia:hardcoded"
insert_fact "The Nile River is approximately 6,650 kilometres long." "geography" "varcavia:hardcoded"
insert_fact "The first successful organ transplant was a kidney transplant in 1954." "health" "varcavia:hardcoded"
insert_fact "Penicillin was discovered by Alexander Fleming in 1928." "health" "varcavia:hardcoded"
insert_fact "The human eye can distinguish approximately 10 million different colours." "health" "varcavia:hardcoded"
insert_fact "Carbon dioxide in Earth's atmosphere has exceeded 420 parts per million." "climate" "varcavia:hardcoded"
insert_fact "The average global temperature has risen by about 1.1 degrees Celsius since pre-industrial times." "climate" "varcavia:hardcoded"
insert_fact "Arctic sea ice has declined by approximately 13 percent per decade since 1979." "climate" "varcavia:hardcoded"
insert_fact "Renewable energy sources generated about 30 percent of global electricity in 2023." "climate" "varcavia:hardcoded"
insert_fact "The ozone layer is located in the stratosphere at 15 to 35 kilometres altitude." "climate" "varcavia:hardcoded"
echo -e "  [Extra]           ${GREEN}done${NC} — $INSERTED inserted"

# ═══════════════════════════════════════
# RISULTATO
# ═══════════════════════════════════════
echo ""
echo "════════════════════════════════════"
echo -e "  Totale:     ${TOTAL}"
echo -e "  Inseriti:   ${GREEN}${INSERTED}${NC}"
echo -e "  Duplicati:  ${YELLOW}${DUPLICATES}${NC}"
echo -e "  Errori:     ${RED}${ERRORS}${NC}"
echo "════════════════════════════════════"
echo ""
echo "Verifica: curl -s $BASE_URL/api/v1/node/status"
