from model.daylio import Daylio, PreferredMoodIconsIdsForMoodIdsForIconsPack
import sys


def init() -> None:
    Daylio.update_forward_refs()
    PreferredMoodIconsIdsForMoodIdsForIconsPack.update_forward_refs()


def load_json(path: str) -> Daylio:
    return Daylio.parse_file(path)


def merge(daylio1: Daylio, daylio2: Daylio) -> Daylio:
    return daylio1


def main() -> None:
    """
    Merges two daylio json files into one.
    We assume the files have been preprocessed:
        - Same version
        - Extracted from the archive (daylio exports zip files)
        - Converted to JSON (daylio exports base64 files)
    """

    if len(sys.argv) not in (3, 4):
        print("Usage: python main.py <in1.json> <in2.json> [<out.json>]")
        sys.exit(1)

    init()

    old = load_json(sys.argv[1])
    new = load_json(sys.argv[2])

    out = "out.json"
    if len(sys.argv) == 4:
        out = sys.argv[3]

    with open(out, "w") as f:
        merged = merge(old, new)
        out_json: str = merged.json(indent=4,
                                    exclude_none=True,
                                    exclude_unset=True)
        f.write(out_json)


if __name__ == "__main__":
    main()
