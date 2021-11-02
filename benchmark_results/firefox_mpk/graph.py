import csv
import numpy as np
import pickle
from collections import defaultdict
from time import process_time
import matplotlib.pyplot as plt

plt.rcParams.update({'font.size': 24})

NUM_TESTS = 1_000
LINES_PER_TEST = 49_180 
def refactor_file(in_filename, out_filename):
    with open(in_filename) as in_file:
        csv_reader = csv.reader(in_file)
        out_lines = []
        
        # first iteration
        for _ in range(LINES_PER_TEST):
            fn_id, raw_cycles = next(csv_reader)
            cycles = int(raw_cycles)
            out_lines.append([fn_id, cycles])

        # every other iteration
        for _ in range(1, NUM_TESTS):
            for line_i in range(LINES_PER_TEST):
                _, raw_cycles = next(csv_reader)
                cycles = int(raw_cycles)
                out_lines[line_i].append(cycles)

    with open(out_filename, 'w') as out_file:
        csv_writer = csv.writer(out_file)
        csv_writer.writerows(out_lines)

def remove_outliers(data):
    Q1 = np.percentile(data, 25, interpolation='midpoint')
    Q2 = np.percentile(data, 50, interpolation='midpoint')
    Q3 = np.percentile(data, 75, interpolation='midpoint')
    IQR = Q3 - Q1
    valid = 1.5 * IQR
    new_data = [x for x in data if
                ((x <= Q2 + valid) and (x >= Q2 - valid))]
    return new_data

def merge_and_sort(withoutvals, withvals):
    valpairs = (pair for pair in zip(withoutvals, withvals))
    return sorted(valpairs, key=lambda p:sum(p)/len(p))

class ValuesData:
    def __init__(self, values):
        self.values = values
        self.mean = np.mean(values)
        self.std = np.std(values)
    
    def __iter__(self):
        yield from self.values

class LineData:
    def __init__(self, index, fn_name, withoutmpk_values, withmpk_values):
        self.index = index
        self.fn_name = fn_name
        self.orig_withoutmpk_values = ValuesData(withoutmpk_values)
        self.orig_withmpk_values = ValuesData(withmpk_values)

        fixed_withoutmpk_values = remove_outliers(withoutmpk_values)
        fixed_withmpk_values = remove_outliers(withmpk_values)
        self.fixed_withoutmpk_values = ValuesData(fixed_withoutmpk_values)
        self.fixed_withmpk_values = ValuesData(fixed_withmpk_values)
        num_outliers_withoutmpk = len(withoutmpk_values) - len(fixed_withoutmpk_values)
        num_outliers_withmpk = len(withmpk_values) - len(fixed_withmpk_values)
        self.num_outliers_withoutmpk = num_outliers_withoutmpk
        self.num_outliers_withmpk = num_outliers_withmpk

        self.orig_overhead = self.orig_withmpk_values.mean / self.orig_withoutmpk_values.mean
        self.fixed_overhead = self.fixed_withmpk_values.mean / self.fixed_withoutmpk_values.mean

class CollectionData:
    def __init__(self):
        self.lines = []

    def __iter__(self):
        yield from self.lines

    def addline(self, index, fn_name, raw_withoutmpk_values, raw_withmpk_values):
        if index != len(self.lines):
            raise IndexError("enumerate failed us")
        withmpk_values = list(map(int, raw_withmpk_values))
        withoutmpk_values = list(map(int, raw_withoutmpk_values))
        self.lines.append(LineData(index, fn_name, withoutmpk_values, withmpk_values))

if __name__ == '__main__':
    start_time = process_time()

    # in_nompkfile = "example-withoutmpk.csv"
    # in_mpkfile = "example-withmpk.csv"
    # in_filenames = [in_nompkfile, in_mpkfile]
    # fixed_nompkfile = "fixed-withoutmpk.csv"
    # fixed_mpkfile = "fixed-withmpk.csv"
    # fixed_filenames = [fixed_nompkfile, fixed_mpkfile]

    # for i, o in zip(in_filenames, fixed_filenames):
    #     refactor_file(i, o)

    # CHECKPOINT 1, CSVs transformed, saved as fixed-*.csv

    # collectiondata = CollectionData()
    # with open(fixed_nompkfile) as without_file, open(fixed_mpkfile) as with_file:
    #     without_reader = csv.reader(without_file)
    #     with_reader = csv.reader(with_file)
    #     for index, ((without_fn_name, *without_values), (with_fn_name, *with_values)) in enumerate(zip(without_reader, with_reader)):
    #         if without_fn_name != with_fn_name:
    #             raise ValueError("dude we're screwed")
    #         collectiondata.addline(index, with_fn_name, without_values, with_values)

    pickle_filename = "collection.pickle"
    # with open(pickle_filename, 'wb') as pickle_file:
    #     pickle.dump(collectiondata, pickle_file)

    # CHECKPOINT 2, mean and stdev data collected, saved as collection.pickle

    with open(pickle_filename, 'rb') as pickle_file:
        collectiondata = pickle.load(pickle_file)
    
    # orig_overheads = [l.orig_overhead for l in collectiondata]
    # orig_parser_overheads = [l.orig_overhead for l in collectiondata if l.fn_name == "Parser::Parse"]
    # fixed_before_overheads = [l.fixed_overhead for l in collectiondata]
    
    

    orig_without_means = [l.orig_withoutmpk_values.mean for l in collectiondata if l.fn_name == "Parser::Parse"]
    orig_with_means = [l.orig_withmpk_values.mean for l in collectiondata if l.fn_name == "Parser::Parse"]
    orig_before_overheads = [l.orig_overhead for l in collectiondata if l.fn_name == "Parser::Parse"]
    sorted_mean_pairs = merge_and_sort(orig_without_means, orig_with_means)
    sfwom, sfwm = zip(*sorted_mean_pairs)

    fig = plt.figure(figsize=(10,10))
    subplt = fig.add_subplot()
    subplt.plot(*zip(*enumerate(sfwom)), color="r", label="original")
    subplt.plot(*zip(*enumerate(sfwm)), color="b", label="with Galeed")
    subplt.legend(loc="upper left")
    subplt.set_xlabel("function call (sorted by # of cycles taken)")
    subplt.set_ylabel("# of cycles taken")
    # fig.suptitle("Benchmark cycle counts - Parser only")
    fig.savefig(f"firefox_parser_cycles.pdf")

    fig = plt.figure(figsize=(10,10))
    subplt = fig.add_subplot()
    subplt.plot(*zip(*enumerate(sorted(orig_before_overheads))), color="b")
    subplt.set_xlabel("function call (sorted by overhead)")
    subplt.set_ylabel("overhead")
    # fig.suptitle("Overheads - Parser only")
    fig.savefig(f"firefox_parser_overhead.pdf")

    orig_without_means = [l.orig_withoutmpk_values.mean for l in collectiondata]
    orig_with_means = [l.orig_withmpk_values.mean for l in collectiondata]
    orig_before_overheads = [l.orig_overhead for l in collectiondata]
    sorted_mean_pairs = merge_and_sort(orig_without_means, orig_with_means)
    sfwom, sfwm = zip(*sorted_mean_pairs)

    fig = plt.figure(figsize=(10,10))
    subplt = fig.add_subplot()
    subplt.plot(*zip(*enumerate(sfwom)), color="r", label="original")
    subplt.plot(*zip(*enumerate(sfwm)), color="b", label="with Galeed")
    subplt.legend(loc="upper left")
    subplt.set_xlabel("function call (sorted by # of cycles taken)")
    subplt.set_ylabel("# of cycles taken")
    # fig.suptitle("Benchmark cycle counts - All function calls")
    fig.savefig(f"firefox_cycles.pdf")

    fig = plt.figure(figsize=(10,10))
    subplt = fig.add_subplot()
    subplt.plot(*zip(*enumerate(sorted(orig_before_overheads))), color="b")
    subplt.set_xlabel("function call (sorted by overhead)")
    subplt.set_ylabel("overhead")
    # fig.suptitle("Overheads - All function calls")
    fig.savefig(f"firefox_overhead.pdf")

    plt.show()

        
    print(f"Compute time: {process_time() - start_time}")
    
