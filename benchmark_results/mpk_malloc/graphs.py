from os import walk
import matplotlib.pyplot as plt
import numpy as np
from itertools import product

DIRECTORIES = ("with_mpk", "without_mpk")
SUBDIRECTORIES = ("read", "write", "read_write")
CYCLES = ("inner_cycles", "outer_cycles")

plt.rcParams.update({'font.size': 24})

def collect_data():
    data = {cycle_type:
            {subd:
             {d:{} for d in DIRECTORIES}
             for subd in SUBDIRECTORIES}
            for cycle_type in CYCLES}
    # invert subdirectories and directories
    # end goal: data["inner_cycles"]["read"]["with_mpk"][100_000]
    for directory, subdirectory in product(DIRECTORIES, SUBDIRECTORIES):
        _, _, filenames = next(walk(f"./{directory}/{subdirectory}"))
        for filename in filenames:
            count = int(filename.split(".csv")[0])
            inner_cycle_list, outer_cycle_list = [], []
            with open(f"{directory}/{subdirectory}/{filename}") as file:
                for line in file.readlines():
                    inner_cycles, outer_cycles = map(int, line.strip().split(','))
                    inner_cycle_list.append(inner_cycles)
                    outer_cycle_list.append(outer_cycles)
            data["inner_cycles"][subdirectory][directory][count] = inner_cycle_list
            data["outer_cycles"][subdirectory][directory][count] = outer_cycle_list
    return data

# Credit to Nathan Burow
def remove_outliers(data):
    Q1 = np.percentile(data, 25, interpolation='midpoint')
    Q2 = np.percentile(data, 50, interpolation='midpoint')
    Q3 = np.percentile(data, 75, interpolation='midpoint')
    IQR = Q3 - Q1
    valid = 1.5 * IQR
    new_data = [x for x in data if
                ((x <= Q2 + valid) and (x >= Q2 - valid))]
    return new_data

# Borrowed from StackOverflow - https://stackoverflow.com/questions/24788200/calculate-the-cumulative-distribution-function-cdf-in-python
def ecdf(a):
    x, counts = np.unique(a, return_counts=True)
    cusum = np.cumsum(counts)
    return x, cusum / cusum[-1]

# Also borrowed from same StackOverflow - https://stackoverflow.com/questions/24788200/calculate-the-cumulative-distribution-function-cdf-in-python
def plot_ecdf(a, name=""):
    x, y = ecdf(a)
    x = np.insert(x, 0, x[0])
    y = np.insert(y, 0, 0.)
    plt.plot(x, y, label=name, drawstyle='steps-post')
    plt.grid(True)

if __name__ == "__main__":
    data = collect_data()

    titles = {
        "read": "Single Read",
        "write": "Single Write",
        "read_write": "Write then Read",
    }
    # example = data["inner_cycles"]["read_write"]["without_mpk"][1000_000]
    # print(example)
    # print(len(example))
    # plot_ecdf(example["unsafe"], "unsafe")
    # plot_ecdf(example["safe"], "safe")
    # plt.show()

    for cycle_type, io_dict in data.items():
        for io_type, safety_dict in io_dict.items():
            fig = plt.figure(figsize=(10,10))
            subplt = fig.add_subplot()
            bps = []
            for safety in ("without_mpk", "with_mpk"):
                num_runs_dict = safety_dict[safety]
                color = "r" if safety == "without_mpk" else "b"
                filtered_data_collection = []
                stdev_collection = []
                for num_runs, run_data in sorted(num_runs_dict.items()):
                    # print(f"{cycle_type=}, {io_type=}, {num_runs=}, {safety=}")
                    mean = np.mean(run_data)
                    stdev = np.std(run_data)
                    stdev_collection.append(stdev)

                    filtered_data = remove_outliers(run_data)   # [x for x in run_data if abs(x - mean) <= stdev]
                    # print(len(run_data) - len(filtered_data))
                    filtered_data_collection.append(filtered_data)
                    # print(f"{mean=}, {stdev=}")
                    # print(f"num_outliers = {num_runs - len(filtered_data)}")
                # print(f"{safety=} {stdev_collection=}")
                bp = subplt.boxplot(filtered_data_collection, sym=f"{color}o")
                bps.append(bp)
                for prop in ['boxes', 'whiskers', 'fliers', 'medians', 'caps']:
                    plt.setp(bp[prop], color=color)
            subplt.legend([bps[1]["boxes"][0], bps[0]["boxes"][0]], ['with MPK', 'without MPK'], loc='center right')
            subplt.set_xticks(list(range(1, len(num_runs_dict) + 1)))
            subplt.set_xticklabels([x//100_000 for x in sorted(num_runs_dict.keys())])
            subplt.set_xlabel("# of times operation was performed (x100k)")
            subplt.set_ylabel("# of cycles taken per operation")
            # fig.suptitle(titles[io_type])
            fig.savefig(f"heap_mpk_micro_{io_type}.pdf")
    plt.show()

# Box and whisker plots
# Label everything
# Remove outliers (> 2 stdevs) after looking at data
# Recalculate after removing outliers
# Mean, mean of means, stdev
