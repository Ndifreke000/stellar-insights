import { render, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import { FormProvider, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { FormField, FormSelect, FormCheckboxGroup } from "../ui/FormField";
import { VALIDATION_MESSAGES } from "@/lib/schemas";

// Test wrapper component
const TestForm = ({ children, schema, defaultValues = {} }) => {
  const methods = useForm({
    resolver: zodResolver(schema),
    defaultValues,
    mode: "onChange",
  });

  return (
    <FormProvider {...methods}>
      <form onSubmit={vi.fn()}>{children}</form>
    </FormProvider>
  );
};

describe("FormField Component", () => {
  const testSchema = z.object({
    testField: z.string().min(1, VALIDATION_MESSAGES.REQUIRED),
  });

  it("should render field with label", () => {
    render(
      <TestForm schema={testSchema}>
        <FormField name="testField" label="Test Field" />
      </TestForm>
    );

    expect(screen.getByLabelText("Test Field")).toBeInTheDocument();
  });

  it("should show validation error", async () => {
    const user = userEvent.setup();
    render(
      <TestForm schema={testSchema}>
        <FormField name="testField" label="Test Field" />
      </TestForm>
    );

    const input = screen.getByLabelText("Test Field");
    await user.click(input);
    await user.tab(); // Blur to trigger validation

    await waitFor(() => {
      expect(screen.getByText(VALIDATION_MESSAGES.REQUIRED)).toBeInTheDocument();
    });
  });

  it("should show description when provided", () => {
    render(
      <TestForm schema={testSchema}>
        <FormField 
          name="testField" 
          label="Test Field" 
          description="This is a description" 
        />
      </TestForm>
    );

    expect(screen.getByText("This is a description")).toBeInTheDocument();
  });

  it("should mark required fields with asterisk", () => {
    render(
      <TestForm schema={testSchema}>
        <FormField name="testField" label="Test Field" required />
      </TestForm>
    );

    expect(screen.getByText("*")).toBeInTheDocument();
  });

  it("should have proper accessibility attributes", () => {
    render(
      <TestForm schema={testSchema}>
        <FormField name="testField" label="Test Field" />
      </TestForm>
    );

    const input = screen.getByLabelText("Test Field");
    expect(input).toHaveAttribute("aria-invalid", "false");
  });
});

describe("FormSelect Component", () => {
  const testSchema = z.object({
    testSelect: z.string().min(1, VALIDATION_MESSAGES.REQUIRED),
  });

  const options = [
    { value: "option1", label: "Option 1" },
    { value: "option2", label: "Option 2" },
  ];

  it("should render select with options", () => {
    render(
      <TestForm schema={testSchema}>
        <FormSelect name="testSelect" label="Test Select" options={options} />
      </TestForm>
    );

    expect(screen.getByLabelText("Test Select")).toBeInTheDocument();
    expect(screen.getByText("Option 1")).toBeInTheDocument();
    expect(screen.getByText("Option 2")).toBeInTheDocument();
  });

  it("should show placeholder when provided", () => {
    render(
      <TestForm schema={testSchema}>
        <FormSelect 
          name="testSelect" 
          label="Test Select" 
          options={options} 
          placeholder="Select an option"
        />
      </TestForm>
    );

    expect(screen.getByText("Select an option")).toBeInTheDocument();
  });

  it("should show validation error", async () => {
    const user = userEvent.setup();
    render(
      <TestForm schema={testSchema}>
        <FormSelect name="testSelect" label="Test Select" options={options} />
      </TestForm>
    );

    const select = screen.getByLabelText("Test Select");
    await user.click(select);
    await user.tab(); // Blur to trigger validation

    await waitFor(() => {
      expect(screen.getByText(VALIDATION_MESSAGES.REQUIRED)).toBeInTheDocument();
    });
  });
});

describe("FormCheckboxGroup Component", () => {
  const testSchema = z.object({
    testCheckboxes: z.array(z.string()).min(1, VALIDATION_MESSAGES.SELECT_ROUTE),
  });

  const checkboxOptions = [
    { value: "option1", label: "Option 1" },
    { value: "option2", label: "Option 2" },
    { value: "option3", label: "Option 3" },
  ];

  it("should render checkbox group", () => {
    render(
      <TestForm schema={testSchema} defaultValues={{ testCheckboxes: [] }}>
        <FormCheckboxGroup 
          name="testCheckboxes" 
          label="Test Checkboxes" 
          options={checkboxOptions} 
        />
      </TestForm>
    );

    expect(screen.getByText("Test Checkboxes")).toBeInTheDocument();
    expect(screen.getByLabelText("Option 1")).toBeInTheDocument();
    expect(screen.getByLabelText("Option 2")).toBeInTheDocument();
    expect(screen.getByLabelText("Option 3")).toBeInTheDocument();
  });

  it("should allow selecting multiple options", async () => {
    const user = userEvent.setup();
    render(
      <TestForm schema={testSchema} defaultValues={{ testCheckboxes: [] }}>
        <FormCheckboxGroup 
          name="testCheckboxes" 
          label="Test Checkboxes" 
          options={checkboxOptions} 
        />
      </TestForm>
    );

    const checkbox1 = screen.getByLabelText("Option 1");
    const checkbox2 = screen.getByLabelText("Option 2");

    await user.click(checkbox1);
    expect(checkbox1).toBeChecked();

    await user.click(checkbox2);
    expect(checkbox2).toBeChecked();
  });

  it("should show validation error when no options selected", async () => {
    const user = userEvent.setup();
    render(
      <TestForm schema={testSchema} defaultValues={{ testCheckboxes: [] }}>
        <FormCheckboxGroup 
          name="testCheckboxes" 
          label="Test Checkboxes" 
          options={checkboxOptions} 
        />
      </TestForm>
    );

    // Trigger validation by interacting with the form
    const checkbox1 = screen.getByLabelText("Option 1");
    await user.click(checkbox1);
    await user.click(checkbox1); // Uncheck to trigger validation

    await waitFor(() => {
      expect(screen.getByText(VALIDATION_MESSAGES.SELECT_ROUTE)).toBeInTheDocument();
    });
  });

  it("should show descriptions when provided", () => {
    const optionsWithDescriptions = [
      { value: "option1", label: "Option 1", description: "Description 1" },
      { value: "option2", label: "Option 2", description: "Description 2" },
    ];

    render(
      <TestForm schema={testSchema} defaultValues={{ testCheckboxes: [] }}>
        <FormCheckboxGroup 
          name="testCheckboxes" 
          label="Test Checkboxes" 
          options={optionsWithDescriptions} 
        />
      </TestForm>
    );

    expect(screen.getByText("Description 1")).toBeInTheDocument();
    expect(screen.getByText("Description 2")).toBeInTheDocument();
  });
});

describe("Accessibility", () => {
  it("should have proper ARIA attributes for errors", async () => {
    const user = userEvent.setup();
    const testSchema = z.object({
      testField: z.string().min(1, VALIDATION_MESSAGES.REQUIRED),
    });

    render(
      <TestForm schema={testSchema}>
        <FormField name="testField" label="Test Field" />
      </TestForm>
    );

    const input = screen.getByLabelText("Test Field");
    await user.click(input);
    await user.tab(); // Blur to trigger validation

    await waitFor(() => {
      expect(input).toHaveAttribute("aria-invalid", "true");
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
  });

  it("should have proper ARIA attributes for checkbox groups", () => {
    const testSchema = z.object({
      testCheckboxes: z.array(z.string()),
    });

    render(
      <TestForm schema={testSchema} defaultValues={{ testCheckboxes: [] }}>
        <FormCheckboxGroup 
          name="testCheckboxes" 
          label="Test Checkboxes" 
          options={[{ value: "option1", label: "Option 1" }]} 
        />
      </TestForm>
    );

    const group = screen.getByRole("group");
    expect(group).toBeInTheDocument();
    expect(group).toHaveAttribute("aria-labelledby");
  });
});
