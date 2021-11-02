from os import walk
import matplotlib.pyplot as plt
import numpy as np

DIRECTORIES = ("read_write", "read_only", "write_only")

plt.rcParams.update({'font.size': 24})

def collect_data():
    data = {}
    for directory in DIRECTORIES:
        directory_data = {}
        _, _, filenames = next(walk(f"./{directory}"))
        for filename in filenames:
            file_data = {"unsafe": [], "safe": []}
            count = int(filename.split(".csv")[0])
            with open(f"{directory}/{filename}") as file:
                for line in file.readlines():
                    unsafe_num, safe_num = map(int, line.strip().split(','))
                    file_data["unsafe"].append(unsafe_num)
                    file_data["safe"].append(safe_num)
            directory_data[count] = file_data
        data[directory] = directory_data
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
    # example = data["read_write"][1_000_000]
    # plot_ecdf(example["unsafe"], "unsafe")
    # plot_ecdf(example["safe"], "safe")
    # plt.show()

    titles = {
        "read_only": "Single Read",
        "write_only": "Single Write",
        "read_write": "Write then Read",
    }

    for directory, num_runs_dict in data.items():
        fig = plt.figure(figsize=(10,10))
        subplt = fig.add_subplot()
        bps = []
        for safety in ("unsafe", "safe"):
            color = "r" if safety == "unsafe" else "b"
            filtered_data_collection = []
            stdev_collection = []
            for num_runs, run_data_dict in sorted(num_runs_dict.items()):
                run_data = run_data_dict[safety]
                # print(f"{directory=}, {num_runs=}, {safety=}")
                mean = np.mean(run_data)
                stdev = np.std(run_data)
                stdev_collection.append(stdev)

                filtered_data = remove_outliers(run_data)   # [x for x in run_data if abs(x - mean) <= stdev]
                print(len(run_data) - len(filtered_data))
                filtered_data_collection.append(filtered_data)
                # print(f"{mean=}, {stdev=}")
                # print(f"num_outliers = {num_runs - len(filtered_data)}")
            print(f"{safety=} {stdev_collection=}")
            bp = subplt.boxplot(filtered_data_collection, sym=f"{color}o")
            bps.append(bp)
            for prop in ['boxes', 'whiskers', 'fliers', 'medians', 'caps']:
                plt.setp(bp[prop], color=color)
        subplt.legend([bps[1]["boxes"][0], bps[0]["boxes"][0]], ['using pseudo-pointers', 'using pointers'], loc='center right')
        subplt.set_xticks(list(range(1, len(num_runs_dict) + 1)))
        subplt.set_xticklabels([x//100_000 for x in sorted(num_runs_dict.keys())])
        subplt.set_xlabel("# of times operation was performed (x100k)")
        subplt.set_ylabel("# of cycles taken per operation")
        # fig.suptitle(titles[directory])
        fig.savefig(f"pseudo_micro_{directory}.pdf")
    plt.show()

# Box and whisker plots
# Label everything
# Remove outliers (> 2 stdevs) after looking at data
# Recalculate after removing outliers
# Mean, mean of means, stdev